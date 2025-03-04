use std::borrow::Cow;
use std::error;
use std::ffi::{c_void, CStr};
use std::fmt;
use std::ptr::NonNull;

use crate::class;
use crate::class_registry::ClassRegistry;
use crate::convert::BoxUnboxVmValue;
use crate::core::ConvertMut;
use crate::error::{Error, RubyException};
use crate::extn::core::exception::{NameError, ScriptError};
use crate::module;
use crate::sys;
use crate::Artichoke;

/// Typedef for an mruby free function for an [`mrb_value`](sys::mrb_value) with
/// `tt` [`MRB_TT_DATA`](sys::mrb_vtype::MRB_TT_DATA).
pub type Free = unsafe extern "C" fn(mrb: *mut sys::mrb_state, data: *mut c_void);

/// A generic implementation of a [`Free`] function for
/// [`mrb_value`](sys::mrb_value)s that store an owned copy of a [`Box`] smart
/// pointer.
///
/// This function ultimately calls [`Box::from_raw`] on the data pointer and
/// drops the resulting [`Box`].
///
/// # Safety
///
/// This function assumes that the data pointer is to an [`Box`]`<T>` created by
/// [`Box::into_raw`]. This fuction bounds `T` by [`BoxUnboxVmValue`] which
/// boxes `T` for the mruby VM like this.
///
/// This function assumes it is called by the mruby VM as a free function for
/// an [`MRB_TT_DATA`](sys::mrb_vtype::MRB_TT_DATA).
pub unsafe extern "C" fn box_unbox_free<T>(_mrb: *mut sys::mrb_state, data: *mut c_void)
where
    T: 'static + BoxUnboxVmValue,
{
    if data.is_null() {
        error!("Received null pointer in box_unbox_free<{}>: {:p}", T::RUBY_TYPE, data);
        eprintln!("Received null pointer in box_unbox_free<{}>: {:p}", T::RUBY_TYPE, data);
    }
    T::free(data);
}

#[cfg(test)]
mod free_test {
    use crate::convert::HeapAllocatedData;

    fn prototype(_func: super::Free) {}

    #[derive(Default, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    struct Data(String);

    impl HeapAllocatedData for Data {
        const RUBY_TYPE: &'static str = "Data";
    }

    #[test]
    fn free_prototype() {
        prototype(super::box_unbox_free::<Data>);
    }
}

/// Typedef for a method exposed in the mruby interpreter.
///
/// This function signature is used for all types of mruby methods, including
/// instance methods, class methods, singleton methods, and global methods.
///
/// `slf` is the method receiver, e.g. `s` in the following invocation of
/// `String#start_with?`.
///
/// ```ruby
/// s = 'artichoke crate'
/// s.start_with?('artichoke')
/// ```
///
/// To extract method arguments, use [`mrb_get_args!`] and the supplied
/// interpreter.
pub type Method = unsafe extern "C" fn(mrb: *mut sys::mrb_state, slf: sys::mrb_value) -> sys::mrb_value;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ClassScope {
    name: String,
    cstring: Box<CStr>,
    enclosing_scope: Option<Box<EnclosingRubyScope>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ModuleScope {
    name: String,
    cstring: Box<CStr>,
    name_symbol: u32,
    enclosing_scope: Option<Box<EnclosingRubyScope>>,
}

/// Typesafe wrapper for the [`RClass *`](sys::RClass) of the enclosing scope
/// for an mruby `Module` or `Class`.
///
/// In Ruby, classes and modules can be defined inside of another class or
/// module. mruby only supports resolving [`RClass`](sys::RClass) pointers
/// relative to an enclosing scope. This can be the top level with
/// [`mrb_class_get`](sys::mrb_class_get) and
/// [`mrb_module_get`](sys::mrb_module_get) or it can be under another class
/// with [`mrb_class_get_under`](sys::mrb_class_get_under) or module with
/// [`mrb_module_get_under`](sys::mrb_module_get_under).
///
/// Because there is no C API to resolve class and module names directly, each
/// class-like holds a reference to its enclosing scope so it can recursively
/// resolve its enclosing [`RClass *`](sys::RClass).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum EnclosingRubyScope {
    /// Reference to a Ruby `Class` enclosing scope.
    Class(ClassScope),
    /// Reference to a Ruby `Module` enclosing scope.
    Module(ModuleScope),
}

impl EnclosingRubyScope {
    /// Factory for [`EnclosingRubyScope::Class`] that clones a [`class::Spec`].
    ///
    /// This function is useful when extracting an enclosing scope from the
    /// class registry.
    #[must_use]
    pub fn class(spec: &class::Spec) -> Self {
        let cstring = spec.name_c_str();
        // Safety
        //
        // This CStr is being constructed with another CStr including the nul
        // byte.
        let cstring = unsafe { CStr::from_bytes_with_nul_unchecked(cstring.to_bytes_with_nul()) };
        Self::Class(ClassScope {
            name: String::from(spec.name()),
            cstring: cstring.into(),
            enclosing_scope: spec.enclosing_scope().map(Clone::clone).map(Box::new),
        })
    }

    /// Factory for [`EnclosingRubyScope::Module`] that clones a
    /// [`module::Spec`].
    ///
    /// This function is useful when extracting an enclosing scope from the
    /// module registry.
    #[must_use]
    pub fn module(spec: &module::Spec) -> Self {
        let cstring = spec.name_c_str();
        // Safety
        //
        // This CStr is being constructed with another CStr including the nul
        // byte.
        let cstring = unsafe { CStr::from_bytes_with_nul_unchecked(cstring.to_bytes_with_nul()) };
        Self::Module(ModuleScope {
            name: String::from(spec.name()),
            cstring: cstring.into(),
            name_symbol: spec.name_symbol(),
            enclosing_scope: spec.enclosing_scope().map(Clone::clone).map(Box::new),
        })
    }

    /// Resolve the [`RClass *`](sys::RClass) of the wrapped class or module.
    ///
    /// Return [`None`] if the class-like has no [`EnclosingRubyScope`].
    ///
    /// The current implemention results in recursive calls to this function
    /// for each enclosing scope.
    ///
    /// # Safety
    ///
    /// This function must be called within an [`Artichoke::with_ffi_boundary`]
    /// closure because the FFI APIs called in this function may require access
    /// to the Artichoke [`State](crate::state::State).
    pub unsafe fn rclass(&self, mrb: *mut sys::mrb_state) -> Option<NonNull<sys::RClass>> {
        match self {
            Self::Class(scope) => {
                let enclosing_scope = scope.enclosing_scope.clone().map(|scope| *scope);
                class::Rclass::new(scope.cstring.clone(), enclosing_scope).resolve(mrb)
            }
            Self::Module(scope) => {
                let enclosing_scope = scope.enclosing_scope.clone().map(|scope| *scope);
                module::Rclass::new(scope.name_symbol, scope.cstring.clone(), enclosing_scope).resolve(mrb)
            }
        }
    }

    /// Get the fully qualified name of the wrapped class or module.
    ///
    /// For example, in the following Ruby code, `C` has an fqname of `A::B::C`.
    ///
    /// ```ruby
    /// module A
    ///   class B
    ///     module C
    ///       CONST = 1
    ///     end
    ///   end
    /// end
    /// ```
    ///
    /// The current implemention results in recursive calls to this function
    /// for each enclosing scope.
    #[must_use]
    pub fn fqname(&self) -> Cow<'_, str> {
        let (name, enclosing_scope) = match self {
            Self::Class(scope) => (&scope.name, &scope.enclosing_scope),
            Self::Module(scope) => (&scope.name, &scope.enclosing_scope),
        };
        if let Some(scope) = enclosing_scope {
            let mut fqname = String::from(scope.fqname());
            fqname.push_str("::");
            fqname.push_str(name);
            fqname.into()
        } else {
            name.into()
        }
    }
}

#[derive(Default, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstantNameError(Cow<'static, str>);

impl From<&'static str> for ConstantNameError {
    fn from(name: &'static str) -> Self {
        Self(name.into())
    }
}

impl From<String> for ConstantNameError {
    fn from(name: String) -> Self {
        Self(name.into())
    }
}

impl From<Cow<'static, str>> for ConstantNameError {
    fn from(name: Cow<'static, str>) -> Self {
        Self(name)
    }
}

impl ConstantNameError {
    #[must_use]
    pub const fn new() -> Self {
        Self(Cow::Borrowed(""))
    }
}

impl fmt::Display for ConstantNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Invalid constant name contained a NUL byte")
    }
}

impl error::Error for ConstantNameError {}

impl RubyException for ConstantNameError {
    fn message(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(b"Invalid constant name contained a NUL byte")
    }

    fn name(&self) -> Cow<'_, str> {
        "NameError".into()
    }

    fn vm_backtrace(&self, interp: &mut Artichoke) -> Option<Vec<Vec<u8>>> {
        let _ = interp;
        None
    }

    fn as_mrb_value(&self, interp: &mut Artichoke) -> Option<sys::mrb_value> {
        let message = interp.convert_mut(self.message());
        let value = interp.new_instance::<NameError>(&[message]).ok().flatten()?;
        Some(value.inner())
    }
}

impl From<ConstantNameError> for Error {
    fn from(exception: ConstantNameError) -> Self {
        Self::from(Box::<dyn RubyException>::from(exception))
    }
}

impl From<Box<ConstantNameError>> for Error {
    fn from(exception: Box<ConstantNameError>) -> Self {
        Self::from(Box::<dyn RubyException>::from(exception))
    }
}

impl From<ConstantNameError> for Box<dyn RubyException> {
    fn from(exception: ConstantNameError) -> Box<dyn RubyException> {
        Box::new(exception)
    }
}

impl From<Box<ConstantNameError>> for Box<dyn RubyException> {
    fn from(exception: Box<ConstantNameError>) -> Box<dyn RubyException> {
        exception
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotDefinedError {
    EnclosingScope(Cow<'static, str>),
    Super(Cow<'static, str>),
    Class(Cow<'static, str>),
    Method(Cow<'static, str>),
    Module(Cow<'static, str>),
    GlobalConstant(Cow<'static, str>),
    ClassConstant(Cow<'static, str>),
    ModuleConstant(Cow<'static, str>),
}

impl NotDefinedError {
    pub fn enclosing_scope<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::EnclosingScope(item.into())
    }

    pub fn super_class<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::Super(item.into())
    }

    pub fn class<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::Class(item.into())
    }

    pub fn method<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::Method(item.into())
    }

    pub fn module<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::Module(item.into())
    }

    pub fn global_constant<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::GlobalConstant(item.into())
    }

    pub fn class_constant<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::ClassConstant(item.into())
    }

    pub fn module_constant<T>(item: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        Self::ModuleConstant(item.into())
    }

    #[must_use]
    pub fn fqdn(&self) -> &str {
        match self {
            Self::EnclosingScope(ref fqdn)
            | Self::Super(ref fqdn)
            | Self::Class(ref fqdn)
            | Self::Module(ref fqdn) => fqdn.as_ref(),
            Self::GlobalConstant(ref name)
            | Self::ClassConstant(ref name)
            | Self::Method(ref name)
            | Self::ModuleConstant(ref name) => name.as_ref(),
        }
    }

    #[must_use]
    pub const fn item_type(&self) -> &str {
        match self {
            Self::EnclosingScope(_) => "enclosing scope",
            Self::Super(_) => "super class",
            Self::Class(_) => "class",
            Self::Method(_) => "method",
            Self::Module(_) => "module",
            Self::GlobalConstant(_) => "global constant",
            Self::ClassConstant(_) => "class constant",
            Self::ModuleConstant(_) => "module constant",
        }
    }
}

impl fmt::Display for NotDefinedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.item_type())?;
        f.write_str(" '")?;
        f.write_str(self.fqdn())?;
        f.write_str("' not defined")?;
        Ok(())
    }
}

impl error::Error for NotDefinedError {}

impl RubyException for NotDefinedError {
    fn message(&self) -> Cow<'_, [u8]> {
        let mut message = String::from(self.item_type());
        message.push(' ');
        message.push_str(self.fqdn());
        message.push_str(" not defined");
        message.into_bytes().into()
    }

    fn name(&self) -> Cow<'_, str> {
        "ScriptError".into()
    }

    fn vm_backtrace(&self, interp: &mut Artichoke) -> Option<Vec<Vec<u8>>> {
        let _ = interp;
        None
    }

    fn as_mrb_value(&self, interp: &mut Artichoke) -> Option<sys::mrb_value> {
        let message = interp.convert_mut(self.message());
        let value = interp.new_instance::<ScriptError>(&[message]).ok().flatten()?;
        Some(value.inner())
    }
}

impl From<NotDefinedError> for Error {
    fn from(exception: NotDefinedError) -> Self {
        Self::from(Box::<dyn RubyException>::from(exception))
    }
}

impl From<Box<NotDefinedError>> for Error {
    fn from(exception: Box<NotDefinedError>) -> Self {
        Self::from(Box::<dyn RubyException>::from(exception))
    }
}

impl From<NotDefinedError> for Box<dyn RubyException> {
    fn from(exception: NotDefinedError) -> Box<dyn RubyException> {
        Box::new(exception)
    }
}

impl From<Box<NotDefinedError>> for Box<dyn RubyException> {
    fn from(exception: Box<NotDefinedError>) -> Box<dyn RubyException> {
        exception
    }
}

#[cfg(test)]
mod tests {
    mod fqname {
        use crate::test::prelude::*;

        /// `A`
        #[derive(Debug)]
        struct Root;

        /// `A::B`
        #[derive(Debug)]
        struct ModuleUnderRoot;

        /// `A::C`
        #[derive(Debug)]
        struct ClassUnderRoot;

        /// `A::B::D`
        #[derive(Debug)]
        struct ClassUnderModule;

        /// `A::C::E`
        #[derive(Debug)]
        struct ModuleUnderClass;

        /// `A::C::F`
        #[derive(Debug)]
        struct ClassUnderClass;

        #[test]
        fn integration_test() {
            // Setup: define module and class hierarchy
            let mut interp = interpreter().unwrap();
            let root = module::Spec::new(&mut interp, "A", None).unwrap();
            let mod_under_root = module::Spec::new(&mut interp, "B", Some(EnclosingRubyScope::module(&root))).unwrap();
            let cls_under_root = class::Spec::new("C", Some(EnclosingRubyScope::module(&root)), None).unwrap();
            let cls_under_mod =
                class::Spec::new("D", Some(EnclosingRubyScope::module(&mod_under_root)), None).unwrap();
            let mod_under_cls =
                module::Spec::new(&mut interp, "E", Some(EnclosingRubyScope::class(&cls_under_root))).unwrap();
            let cls_under_cls = class::Spec::new("F", Some(EnclosingRubyScope::class(&cls_under_root)), None).unwrap();
            module::Builder::for_spec(&mut interp, &root).define().unwrap();
            module::Builder::for_spec(&mut interp, &mod_under_root)
                .define()
                .unwrap();
            class::Builder::for_spec(&mut interp, &cls_under_root).define().unwrap();
            class::Builder::for_spec(&mut interp, &cls_under_mod).define().unwrap();
            module::Builder::for_spec(&mut interp, &mod_under_cls).define().unwrap();
            class::Builder::for_spec(&mut interp, &cls_under_cls).define().unwrap();
            interp.def_module::<Root>(root).unwrap();
            interp.def_module::<ModuleUnderRoot>(mod_under_root).unwrap();
            interp.def_class::<ClassUnderRoot>(cls_under_root).unwrap();
            interp.def_class::<ClassUnderModule>(cls_under_mod).unwrap();
            interp.def_module::<ModuleUnderClass>(mod_under_cls).unwrap();
            interp.def_class::<ClassUnderClass>(cls_under_cls).unwrap();

            let root = interp.module_spec::<Root>().unwrap().unwrap();
            assert_eq!(root.fqname().as_ref(), "A");
            let mod_under_root = interp.module_spec::<ModuleUnderRoot>().unwrap().unwrap();
            assert_eq!(mod_under_root.fqname().as_ref(), "A::B");
            let cls_under_root = interp.class_spec::<ClassUnderRoot>().unwrap().unwrap();
            assert_eq!(cls_under_root.fqname().as_ref(), "A::C");
            let cls_under_mod = interp.class_spec::<ClassUnderModule>().unwrap().unwrap();
            assert_eq!(cls_under_mod.fqname().as_ref(), "A::B::D");
            let mod_under_cls = interp.module_spec::<ModuleUnderClass>().unwrap().unwrap();
            assert_eq!(mod_under_cls.fqname().as_ref(), "A::C::E");
            let cls_under_cls = interp.class_spec::<ClassUnderClass>().unwrap().unwrap();
            assert_eq!(cls_under_cls.fqname().as_ref(), "A::C::F");
        }
    }

    mod functional {
        use crate::test::prelude::*;

        #[derive(Debug)]
        struct Class;

        #[derive(Debug)]
        struct Module;

        extern "C" fn value(_mrb: *mut sys::mrb_state, slf: sys::mrb_value) -> sys::mrb_value {
            unsafe {
                match slf.tt {
                    sys::mrb_vtype::MRB_TT_CLASS => sys::mrb_sys_fixnum_value(8),
                    sys::mrb_vtype::MRB_TT_MODULE => sys::mrb_sys_fixnum_value(27),
                    sys::mrb_vtype::MRB_TT_OBJECT => sys::mrb_sys_fixnum_value(64),
                    _ => sys::mrb_sys_fixnum_value(125),
                }
            }
        }

        #[test]
        fn define_method() {
            let mut interp = interpreter().unwrap();
            let class = class::Spec::new("DefineMethodTestClass", None, None).unwrap();
            class::Builder::for_spec(&mut interp, &class)
                .add_method("value", value, sys::mrb_args_none())
                .unwrap()
                .add_self_method("value", value, sys::mrb_args_none())
                .unwrap()
                .define()
                .unwrap();
            interp.def_class::<Class>(class).unwrap();
            let module = module::Spec::new(&mut interp, "DefineMethodTestModule", None).unwrap();
            module::Builder::for_spec(&mut interp, &module)
                .add_method("value", value, sys::mrb_args_none())
                .unwrap()
                .add_self_method("value", value, sys::mrb_args_none())
                .unwrap()
                .define()
                .unwrap();
            interp.def_module::<Module>(module).unwrap();

            let _ = interp
                .eval(b"class DynamicTestClass; include DefineMethodTestModule; extend DefineMethodTestModule; end")
                .unwrap();
            let _ = interp
                .eval(b"module DynamicTestModule; extend DefineMethodTestModule; end")
                .unwrap();

            let result = interp.eval(b"DefineMethodTestClass.new.value").unwrap();
            let result = result.try_into::<i64>(&interp).unwrap();
            assert_eq!(result, 64);
            let result = interp.eval(b"DefineMethodTestClass.value").unwrap();
            let result = result.try_into::<i64>(&interp).unwrap();
            assert_eq!(result, 8);
            let result = interp.eval(b"DefineMethodTestModule.value").unwrap();
            let result = result.try_into::<i64>(&interp).unwrap();
            assert_eq!(result, 27);
            let result = interp.eval(b"DynamicTestClass.new.value").unwrap();
            let result = result.try_into::<i64>(&interp).unwrap();
            assert_eq!(result, 64);
            let result = interp.eval(b"DynamicTestClass.value").unwrap();
            let result = result.try_into::<i64>(&interp).unwrap();
            assert_eq!(result, 8);
            let result = interp.eval(b"DynamicTestModule.value").unwrap();
            let result = result.try_into::<i64>(&interp).unwrap();
            assert_eq!(result, 27);
        }
    }
}
