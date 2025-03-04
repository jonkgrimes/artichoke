//! Artichoke CLI entrypoint.
//!
//! Artichoke's version of the `ruby` CLI. This module is exported as the
//! `artichoke` binary.

use std::error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use termcolor::WriteColor;

use crate::backend::ffi;
use crate::backend::state::parser::Context;
use crate::backend::string::format_unicode_debug_into;
use crate::backtrace;
use crate::filename::INLINE_EVAL_SWITCH;
use crate::prelude::*;

/// Command line arguments for Artichoke `ruby` frontend.
#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Args {
    /// print the copyright
    copyright: bool,
    /// one line of script. Several -e's allowed. Omit \[programfile\]
    commands: Vec<OsString>,
    /// file whose contents will be read into the `$fixture` global
    fixture: Option<PathBuf>,
    programfile: Option<PathBuf>,
    /// Trailing positional arguments.
    ///
    /// Requires `programfile` to be present.
    argv: Vec<OsString>,
}

impl Args {
    /// Construct a new, empty `Args`.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            copyright: false,
            commands: Vec::new(),
            fixture: None,
            programfile: None,
            argv: Vec::new(),
        }
    }

    /// Add a parsed copyright flag to this `Args`.
    #[must_use]
    pub fn with_copyright(mut self, copyright: bool) -> Self {
        self.copyright = copyright;
        self
    }

    /// Add a parsed set of `-e` inline eval commands to this `Args`.
    #[must_use]
    pub fn with_commands(mut self, commands: Vec<OsString>) -> Self {
        self.commands = commands;
        self
    }

    /// Add a parsed fixture path to this `Args`.
    #[must_use]
    pub fn with_fixture(mut self, fixture: Option<PathBuf>) -> Self {
        self.fixture = fixture;
        self
    }

    /// Add a parsed program file path to this `Args`.
    #[must_use]
    pub fn with_programfile(mut self, programfile: Option<PathBuf>) -> Self {
        self.programfile = programfile;
        self
    }

    /// Add a parsed ARGV to this `Args`.
    #[must_use]
    pub fn with_argv(mut self, argv: Vec<OsString>) -> Self {
        self.argv = argv;
        self
    }
}

/// Main entrypoint for Artichoke's version of the `ruby` CLI.
///
/// # Errors
///
/// If an exception is raised on the interpreter, then an error is returned.
pub fn run<R, W>(args: Args, input: R, error: W) -> Result<Result<(), ()>, Box<dyn error::Error>>
where
    R: io::Read,
    W: io::Write + WriteColor,
{
    let mut interp = crate::interpreter()?;
    let result = entrypoint(&mut interp, args, input, error);
    interp.close();
    result
}

/// Main entrypoint for Artichoke's version of the `ruby` CLI.
///
/// # Errors
///
/// If an exception is raised on the interpreter, then an error is returned.
pub fn entrypoint<R, W>(
    interp: &mut Artichoke,
    args: Args,
    mut input: R,
    error: W,
) -> Result<Result<(), ()>, Box<dyn error::Error>>
where
    R: io::Read,
    W: io::Write + WriteColor,
{
    if args.copyright {
        let _ = interp.eval(b"puts RUBY_COPYRIGHT")?;
        return Ok(Ok(()));
    }

    // Inject ARGV global.
    let mut ruby_program_argv = Vec::new();
    for argument in &args.argv {
        let argument = ffi::os_str_to_bytes(argument)?;
        let mut argument = interp.try_convert_mut(argument)?;
        argument.freeze(interp)?;
        ruby_program_argv.push(argument);
    }
    let ruby_program_argv = interp.try_convert_mut(ruby_program_argv)?;
    interp.define_global_constant("ARGV", ruby_program_argv)?;

    if !args.commands.is_empty() {
        execute_inline_eval(interp, error, args.commands, args.fixture.as_deref())
    } else if let Some(programfile) = args.programfile.filter(|file| file != Path::new("-")) {
        execute_program_file(interp, error, programfile.as_path(), args.fixture.as_deref())
    } else {
        let mut program = vec![];
        input
            .read_to_end(&mut program)
            .map_err(|_| IOError::from("Could not read program from STDIN"))?;
        if let Err(ref exc) = interp.eval(program.as_slice()) {
            backtrace::format_cli_trace_into(error, interp, exc)?;
            return Ok(Err(()));
        }
        Ok(Ok(()))
    }
}

fn execute_inline_eval<W>(
    interp: &mut Artichoke,
    error: W,
    commands: Vec<OsString>,
    fixture: Option<&Path>,
) -> Result<Result<(), ()>, Box<dyn error::Error>>
where
    W: io::Write + WriteColor,
{
    interp.pop_context()?;
    // Safety:
    //
    // - `Context::new_unchecked` requires that its argument has no NUL bytes.
    // - `INLINE_EVAL_SWITCH` is controlled by this crate.
    // - A test asserts that `INLINE_EVAL_SWITCH` has no NUL bytes.
    let context = unsafe { Context::new_unchecked(INLINE_EVAL_SWITCH) };
    interp.push_context(context)?;
    if let Some(fixture) = fixture {
        setup_fixture_hack(interp, fixture)?;
    }
    for command in commands {
        if let Err(ref exc) = interp.eval_os_str(command.as_os_str()) {
            backtrace::format_cli_trace_into(error, interp, exc)?;
            // short circuit, but don't return an error since we already printed it
            return Ok(Err(()));
        }
        interp.add_fetch_lineno(1)?;
    }
    Ok(Ok(()))
}

fn execute_program_file<W>(
    interp: &mut Artichoke,
    error: W,
    programfile: &Path,
    fixture: Option<&Path>,
) -> Result<Result<(), ()>, Box<dyn error::Error>>
where
    W: io::Write + WriteColor,
{
    if let Some(fixture) = fixture {
        setup_fixture_hack(interp, fixture)?;
    }
    if let Err(ref exc) = interp.eval_file(programfile) {
        backtrace::format_cli_trace_into(error, interp, exc)?;
        return Ok(Err(()));
    }
    Ok(Ok(()))
}

fn load_error<P: AsRef<OsStr>>(file: P, message: &str) -> Result<String, Error> {
    let mut buf = String::from(message);
    buf.push_str(" -- ");
    let path = ffi::os_str_to_bytes(file.as_ref())?;
    format_unicode_debug_into(&mut buf, path)?;
    Ok(buf)
}

// This function exists to provide a workaround for Artichoke not being able to
// read from the local filesystem.
//
// By passing the `--with-fixture PATH` argument, this function loads the file
// at `PATH` into memory and stores it in the interpreter bound to the
// `$fixture` global.
#[inline]
fn setup_fixture_hack<P: AsRef<Path>>(interp: &mut Artichoke, fixture: P) -> Result<(), Error> {
    let data = if let Ok(data) = fs::read(fixture.as_ref()) {
        data
    } else {
        return Err(LoadError::from(load_error(fixture.as_ref(), "No such file or directory")?).into());
    };
    let value = interp.convert_mut(data);
    interp.set_global_variable(&b"$fixture"[..], &value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use termcolor::Ansi;

    use super::{run, Args};

    #[test]
    fn run_with_copyright() {
        let args = Args::empty().with_copyright(true);
        let input = Vec::<u8>::new();
        let mut err = Ansi::new(Vec::new());
        assert!(matches!(run(args, &input[..], &mut err), Ok(Ok(_))));
    }

    #[test]
    fn run_with_programfile_from_stdin() {
        let args = Args::empty().with_programfile(Some(PathBuf::from("-")));
        let input = b"2 + 7";
        let mut err = Ansi::new(Vec::new());
        assert!(matches!(run(args, &input[..], &mut err), Ok(Ok(_))));
    }

    #[test]
    fn run_with_programfile_from_stdin_raise_exception() {
        let args = Args::empty().with_programfile(Some(PathBuf::from("-")));
        let input = b"raise ArgumentError";
        let mut err = Ansi::new(Vec::new());
        assert!(matches!(run(args, &input[..], &mut err), Ok(Err(_))));
    }

    #[test]
    fn run_with_stdin() {
        let args = Args::empty();
        let input = b"2 + 7";
        let mut err = Ansi::new(Vec::new());
        assert!(matches!(run(args, &input[..], &mut err), Ok(Ok(_))));
    }

    #[test]
    fn run_with_stdin_raise_exception() {
        let args = Args::empty();
        let input = b"raise ArgumentError";
        let mut err = Ansi::new(Vec::new());
        assert!(matches!(run(args, &input[..], &mut err), Ok(Err(_))));
    }

    #[test]
    fn run_with_inline_eval() {
        let args = Args::empty().with_commands(vec![OsString::from("2 + 7")]);
        let input = Vec::<u8>::new();
        let mut err = Ansi::new(Vec::new());
        assert!(matches!(run(args, input.as_slice(), &mut err), Ok(Ok(_))));
    }

    #[test]
    fn run_with_inline_eval_raise_exception() {
        let args = Args::empty().with_commands(vec![OsString::from("raise ArgumentError")]);
        let input = Vec::<u8>::new();
        let mut err = Ansi::new(Vec::new());
        assert!(matches!(run(args, &input[..], &mut err), Ok(Err(_))));
    }
}
