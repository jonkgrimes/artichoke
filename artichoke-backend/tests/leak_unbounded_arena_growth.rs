#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! This integration test checks for memory leaks that stem from improper
//! handling of `mrb_state`.
//!
//! Checks for memory leaks stemming from improperly grabage collecting Ruby
//! objects created in C functions, like the call to exc.inspect in
//! [`ArtichokeApi::current_exception`].
//!
//! The test exposes a function that raises an exception with a 1MB `String`
//! message. The test reuses one artichoke interpreter for all `ITERATIONS`.
//!
//! This test calls [`Value::to_s`] and [`Value::to_s_debug'] on a 1MB `String`.
//!
//! If resident memory increases more than 10MB during the test, we likely are
//! leaking memory.
//!
//! This test fails before commit
//! `a450ca7c458d0a4db6fdc60375d8c2c8482c85a7` with a fairly massive leak.

use artichoke_backend::prelude::*;

const ITERATIONS: usize = 100;

#[test]
fn unbounded_arena_growth_leak_current_exception() {
    let mut interp = artichoke_backend::interpreter().unwrap();
    let code = r#"
def bad_code
  raise ArgumentError.new("n" * 1024 * 1024)
end
    "#;
    let _ = interp.eval(code.trim().as_bytes()).unwrap();
    let expected = Some(vec![
        Vec::from(&b"(eval):2:in bad_code"[..]),
        Vec::from(&b"(eval):1"[..]),
    ]);
    for _ in 0..ITERATIONS {
        let code = b"bad_code";
        let mut arena = interp.create_arena_savepoint().unwrap();
        let result = arena.eval(code).unwrap_err();
        let backtrace = result.vm_backtrace(&mut arena);
        assert_eq!(expected, backtrace);
        drop(result);
        arena.restore();
        interp.incremental_gc();
    }
    interp.close();
}

#[test]
fn unbounded_arena_growth_leak_to_s() {
    let mut interp = artichoke_backend::interpreter().unwrap();
    let expected = "a".repeat(1024 * 1024);
    for _ in 0..ITERATIONS {
        let mut arena = interp.create_arena_savepoint().unwrap();
        let result = arena.eval(b"'a' * 1024 * 1024").unwrap();
        let display = result.to_s(&mut arena);
        assert_eq!(display, expected.as_bytes());
        let _ = result;
        arena.restore();
        interp.incremental_gc();
    }
    interp.full_gc();
    interp.close();
}

#[test]
fn unbounded_arena_growth_leak_inspect() {
    let mut interp = artichoke_backend::interpreter().unwrap();

    let mut expected = String::from('"');
    expected.push_str(&"a".repeat(1024 * 1024));
    expected.push('"');
    let expected = expected.into_bytes();

    for _ in 0..ITERATIONS {
        let mut arena = interp.create_arena_savepoint().unwrap();
        let result = arena.eval(b"'a' * 1024 * 1024").unwrap();
        let debug = result.inspect(&mut arena);
        assert_eq!(debug, expected);
        let _ = result;
        arena.restore();
        interp.incremental_gc();
    }
    interp.full_gc();
    interp.close();
}
