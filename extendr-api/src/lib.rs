//!
//! A safe and user friendly R extension interface.
//!
//! * Build rust extensions to R.
//! * Convert R packages to Rust crates.
//!
//! This library aims to provide an interface that will be familiar to
//! first-time users of Rust or indeed any compiled language.
//!
//! See [Robj] for much of the content of this crate.
//! [Robj] provides a safe wrapper for the R object type.
//! 
//! Use attributes and macros to export to R.
//! ```ignore
//! use extendr_api::*;
//! // Export a function or impl to R.
//! #[extendr]
//! fn fred(a: i32) -> i32 {
//!     a + 1
//! }
//!
//! // define exports using extendr_module 
//! extendr_module! {
//!    mod mymodule;
//!    fn fred;    
//! }
//! 
//! ```
//!
//! In R:
//!
//! ```ignore
//! result <- fred(1)
//! ```
//!
//! [Robj] is a wrapper for R objects.
//! The r!() and R!() macros let you build R objects
//! using Rust and R syntax respectively.
//! ```
//! use extendr_api::*;        // Put API in scope.
//! extendr_engine::start_r(); // Start test environment.
//!
//! // An R object with a single string "hello"
//! let character = r!("hello");
//!
//! // An R integer object with a single number 1L.
//! // Note that in Rust, 1 is an integer and 1.0 is a real.
//! let integer = r!(1);
//!
//! // An R real object with a single number 1.
//! // Note that in R, 1 is a real and 1L is an integer.
//! let real = r!(1.0);
//!
//! // An R real vector.
//! let real_vector = r!([1.0, 2.0]);
//!
//! // An R function object.
//! let function = R!(function(x, y) { x + y });
//!
//! // A named list using the list! macro.
//! let list = list!(a = 1, b = 2);
//! ```
//!
//! In Rust, we prefer to use iterators rather than loops.
//!
//! ```
//! use extendr_api::*;        // Put API in scope.
//! extendr_engine::start_r(); // Start test environment.
//!
//! // 1 ..= 100 is the same as 1:100
//! let res = (1 ..= 100).collect_robj();
//! assert_eq!(res.len(), 100);
//!
//! // Rust arrays are zero-indexed so it is more common to use 0 .. 100.
//! let res = (0 .. 100).collect_robj();
//! assert_eq!(res.len(), 100);
//! ```
//!
//! To index a vector, first convert it to a slice and then
//! remember to use 0-based indexing. In Rust, going out of bounds
//! will cause and error (a panic) unlike C++ which may crash.
//! ```
//! use extendr_api::*;        // Put API in scope.
//! extendr_engine::start_r(); // Start test environment.
//!
//! let vals = r!([1.0, 2.0]);
//! let slice = vals.as_real_slice().unwrap();
//! let one = slice[0];
//! let two = slice[1];
//! //let error = slice[2];
//! ```
//!
//! The [R!] macro lets you embed R code in Rust.
//!
//! The [r!] macro converts a rust object to an R object.
//!
//! You can call R functions and primitives using the [call!] macro.
//! ```
//!
//! use extendr_api::*;        // Put API in scope.
//! extendr_engine::start_r(); // Start test environment.
//!
//! // As one R! macro call
//! let confint1 = R!(confint(lm(weight ~ group - 1, PlantGrowth))).unwrap();
//!
//! // As many parameterized calls.
//! let formula = R!(weight ~ group - 1).unwrap();
//! let plant_growth = R!(PlantGrowth).unwrap();
//! let model = call!("lm", formula, plant_growth).unwrap();
//! let confint2 = call!("confint", model).unwrap();
//!
//! assert_eq!(confint1.as_real_vector(), confint2.as_real_vector());
//! ```
//!
//! Rust has a concept of "Owned" and "Borrowed" objects.
//!
//! Owned objects, such as [Vec] and [String] allocate memory
//! which is released when the object lifetime ends.
//!
//! Borrowed objects such as &[i32] and &str are just pointers
//! to annother object's memory and can't live longer than the
//! object they reference.
//!
//! Borrowed objects are much faster than owned objects and use less
//! memory but are used only for temporary access.
//!
//! When we take a slice of an R vector, for example, we need the
//! original R object to be alive or the data will be corrupted.
//!
//! ```
//! use extendr_api::*;        // Put API in scope.
//! extendr_engine::start_r(); // Start test environment.
//!
//! // robj is an "Owned" object that controls the memory allocated.
//! let robj = r!([1, 2, 3]);
//!
//! // slice is a "borrowed" reference to the bytes in robj.
//! // and cannot live longer than robj.
//! let slice = robj.as_integer_slice().unwrap();
//! ```

#![doc(html_logo_url = "https://raw.githubusercontent.com/extendr/extendr/master/extendr-logo-256.png")]

mod lang;
mod error;
mod logical;
mod rmacros;
mod robj;
mod wrapper;
mod thread_safety;

#[cfg(feature = "ndarray")]
mod robj_ndarray;

pub use lang::*;
pub use error::*;
pub use rmacros::*;
pub use robj::*;
pub use wrapper::*;
pub use logical::*;
pub use thread_safety::{single_threaded, handle_panic};

#[cfg(feature = "ndarray")]
pub use robj_ndarray::*;

pub use extendr_macros::*;

#[doc(hidden)]
pub use libR_sys::DllInfo;

#[doc(hidden)]
pub use libR_sys::SEXP;

#[doc(hidden)]
use libR_sys::*;

#[doc(hidden)]
use std::ffi::CString;

/// Generic dynamic error type.
pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[doc(hidden)]
pub struct CallMethod {
    pub call_symbol: std::ffi::CString,
    pub func_ptr: *const u8,
    pub num_args: i32,
}

// Internal function used to implement the .Call interface.
// This is called from the code generated by the #[extendr] attribute.
#[doc(hidden)]
pub unsafe fn register_call_methods(info: *mut libR_sys::DllInfo, methods: &[CallMethod]) {
    let mut rmethods: Vec<_> = methods
        .iter()
        .map(|m| libR_sys::R_CallMethodDef {
            name: m.call_symbol.as_ptr(),
            fun: Some(std::mem::transmute(m.func_ptr)),
            numArgs: m.num_args,
        })
        .collect();

    rmethods.push(libR_sys::R_CallMethodDef {
        name: std::ptr::null(),
        fun: None,
        numArgs: 0,
    });

    libR_sys::R_registerRoutines(
        info,
        std::ptr::null(),
        rmethods.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
    );
    libR_sys::R_useDynamicSymbols(info, 0);
    libR_sys::R_forceSymbols(info, 1);
}

/// Return true if this primitive is NA.
pub trait IsNA {
    fn is_na(&self) -> bool;
}

impl IsNA for f64 {
    fn is_na(&self) -> bool {
        unsafe { R_IsNA(*self) != 0 }
    }
}

impl IsNA for i32 {
    fn is_na(&self) -> bool {
        *self == std::i32::MIN
    }
}

impl IsNA for Bool {
    fn is_na(&self) -> bool {
        self.0 == std::i32::MIN
    }
}

#[doc(hidden)]
pub fn print_r_output<T: Into<Vec<u8>>>(s: T) {
    let cs = CString::new(s).expect("NulError");
    unsafe {
        Rprintf(cs.as_ptr());
    }
}

#[doc(hidden)]
pub fn print_r_error<T: Into<Vec<u8>>>(s: T) {
    let cs = CString::new(s).expect("NulError");
    unsafe {
        REprintf(cs.as_ptr());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as extendr_api;

    use extendr_macros::extendr;
    use extendr_macros::extendr_module;

    #[extendr]
    pub fn inttypes(a: i8, b: u8, c: i16, d: u16, e: i32, f: u32, g: i64, h: u64) {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
        assert_eq!(c, 3);
        assert_eq!(d, 4);
        assert_eq!(e, 5);
        assert_eq!(f, 6);
        assert_eq!(g, 7);
        assert_eq!(h, 8);
    }

    #[extendr]
    pub fn floattypes(a: f32, b: f64) {
        assert_eq!(a, 1.);
        assert_eq!(b, 2.);
    }

    #[extendr]
    pub fn strtypes(a: &str, b: String) {
        assert_eq!(a, "abc");
        assert_eq!(b, "def");
    }

    #[extendr]
    pub fn vectortypes(a: Vec<i32>, b: Vec<f64>) {
        assert_eq!(a, [1, 2, 3]);
        assert_eq!(b, [4., 5., 6.]);
    }

    #[extendr]
    pub fn robjtype(a: Robj) {
        assert_eq!(a, Robj::from(1))
    }

    #[extendr]
    pub fn return_u8() -> u8 {
        123
    }

    #[extendr]
    pub fn return_u16() -> u16 {
        123
    }

    #[extendr]
    pub fn return_u32() -> u32 {
        123
    }

    #[extendr]
    pub fn return_u64() -> u64 {
        123
    }

    #[extendr]
    pub fn return_i8() -> i8 {
        123
    }

    #[extendr]
    pub fn return_i16() -> i16 {
        123
    }

    #[extendr]
    pub fn return_i32() -> i32 {
        123
    }

    #[extendr]
    pub fn return_i64() -> i64 {
        123
    }

    #[extendr]
    pub fn return_f32() -> f32 {
        123.
    }

    #[extendr]
    pub fn return_f64() -> f64 {
        123.
    }

    struct Person {
        pub name: String,
    }

    #[extendr]
    impl Person {
        fn new() -> Self {
            Self {
                name: "".to_string(),
            }
        }

        fn set_name(&mut self, name: &str) {
            self.name = name.to_string();
        }

        fn name(&self) -> &str {
            self.name.as_str()
        }
    }

    #[extendr]
    fn aux_func(_person: &Person) {}

    // Macro to generate exports
    extendr_module! {
        mod my_module;
        fn aux_func;
        impl Person;
    }

    #[test]
    fn export_test() {
        extendr_engine::start_r();
        use super::*;
        // Call the exported functions through their generated C wrappers.
        unsafe {
            wrap__inttypes(
                Robj::from(1).get(),
                Robj::from(2).get(),
                Robj::from(3).get(),
                Robj::from(4).get(),
                Robj::from(5).get(),
                Robj::from(6).get(),
                Robj::from(7).get(),
                Robj::from(8).get(),
            );
            wrap__inttypes(
                Robj::from(1.).get(),
                Robj::from(2.).get(),
                Robj::from(3.).get(),
                Robj::from(4.).get(),
                Robj::from(5.).get(),
                Robj::from(6.).get(),
                Robj::from(7.).get(),
                Robj::from(8.).get(),
            );
            wrap__floattypes(Robj::from(1.).get(), Robj::from(2.).get());
            wrap__floattypes(Robj::from(1).get(), Robj::from(2).get());
            wrap__strtypes(Robj::from("abc").get(), Robj::from("def").get());
            wrap__vectortypes(
                Robj::from(&[1, 2, 3] as &[i32]).get(),
                Robj::from(&[4., 5., 6.] as &[f64]).get(),
            );
            wrap__robjtype(Robj::from(1).get());

            // Note u8 is special as it makes the raw type
            assert_eq!(new_borrowed(wrap__return_u8()), Robj::from(&[123_u8][..]));

            // General integer types.
            assert_eq!(new_borrowed(wrap__return_u16()), Robj::from(123));
            assert_eq!(new_borrowed(wrap__return_u32()), Robj::from(123));
            assert_eq!(new_borrowed(wrap__return_u64()), Robj::from(123));
            assert_eq!(new_borrowed(wrap__return_i8()), Robj::from(123));
            assert_eq!(new_borrowed(wrap__return_i16()), Robj::from(123));
            assert_eq!(new_borrowed(wrap__return_i32()), Robj::from(123));
            assert_eq!(new_borrowed(wrap__return_i64()), Robj::from(123));

            // Floating point types.
            assert_eq!(new_borrowed(wrap__return_f32()), Robj::from(123.));
            assert_eq!(new_borrowed(wrap__return_f64()), Robj::from(123.));
        }
    }

    #[test]
    fn r_output_test() {
        
        // R equivalent
        // > txt_con <- textConnection("test_con", open = "w")
        // > sink(txt_con)
        // > cat("Hello world")
        // > sink()
        // > close(txt_con)
        // > expect_equal(test_con, "Hello world")
        //

        extendr_engine::start_r();
        let txt_con = R!(textConnection("test_con", open = "w")).unwrap();
        call!("sink", &txt_con).unwrap();
        rprintln!("Hello world");
        call!("sink").unwrap();
        call!("close", &txt_con).unwrap();
        let result = R!(test_con).unwrap();
        assert_eq!(result, r!("Hello world"));
    }
}
