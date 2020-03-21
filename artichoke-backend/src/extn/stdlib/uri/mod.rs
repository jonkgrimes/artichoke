use crate::extn::prelude::*;

pub fn init(interp: &mut Artichoke) -> InitializeResult<()> {
    let spec = class::Spec::new("IPSocket", None, None)?;
    interp.0.borrow_mut().def_class::<IpSocket>(spec);

    let spec = class::Spec::new("IPAddr", None, None)?;
    interp.0.borrow_mut().def_class::<IpAddr>(spec);

    let spec = module::Spec::new(interp, "URI", None)?;
    interp.0.borrow_mut().def_module::<Uri>(spec);

    interp.def_rb_source_file(b"uri.rb", &include_bytes!("uri.rb")[..])?;
    interp.def_rb_source_file(b"uri/common.rb", &include_bytes!("uri/common.rb")[..])?;
    interp.def_rb_source_file(b"uri/file.rb", &include_bytes!("uri/file.rb")[..])?;
    interp.def_rb_source_file(b"uri/ftp.rb", &include_bytes!("uri/ftp.rb")[..])?;
    interp.def_rb_source_file(b"uri/generic.rb", &include_bytes!("uri/generic.rb")[..])?;
    interp.def_rb_source_file(b"uri/http.rb", &include_bytes!("uri/http.rb")[..])?;
    interp.def_rb_source_file(b"uri/https.rb", &include_bytes!("uri/https.rb")[..])?;
    interp.def_rb_source_file(b"uri/ldap.rb", &include_bytes!("uri/ldap.rb")[..])?;
    interp.def_rb_source_file(b"uri/ldaps.rb", &include_bytes!("uri/ldaps.rb")[..])?;
    interp.def_rb_source_file(b"uri/mailto.rb", &include_bytes!("uri/mailto.rb")[..])?;
    interp.def_rb_source_file(
        b"uri/rfc2396_parser.rb",
        &include_bytes!("uri/rfc2396_parser.rb")[..],
    )?;
    interp.def_rb_source_file(
        b"uri/rfc3986_parser.rb",
        &include_bytes!("uri/rfc3986_parser.rb")[..],
    )?;

    Ok(())
}

#[derive(Debug)]
pub struct IpSocket;

#[derive(Debug)]
pub struct IpAddr;

#[derive(Debug)]
pub struct Uri;
