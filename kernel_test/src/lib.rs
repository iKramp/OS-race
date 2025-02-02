#![no_std]

use proc_macro::TokenStream;
extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use syn::ItemFn;

static mut TESTS: Vec<String> = Vec::new();
static mut CURR_MOD: String = String::new();

#[proc_macro_attribute]
pub fn kernel_test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let function = input.to_string();

    let input_fn = syn::parse_macro_input!(input as ItemFn);
    let test_fn = input_fn.sig.ident;
    let mut function_full_name = unsafe { CURR_MOD.clone() };
    function_full_name.push_str("::");
    function_full_name.push_str(&test_fn.to_string());

    let mut code = r#"
        #[cfg(feature = "run_tests")]
        pub "#.to_string();
    code.push_str(&function);
    code.push('\n');

    unsafe {
        TESTS.push(function_full_name.to_string());
    }

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn all_tests(_item: TokenStream) -> TokenStream {
    let mut code = "[".to_owned();

    unsafe {
        #[allow(static_mut_refs)]
        for test in &TESTS {
            let function_name = test.split(':').last().unwrap();
            //code = code.add(&format!("({test} as fn() -> bool, \"{function_name}\"),"));
            code.push('(');
            code.push_str(test);
            code.push_str(" as fn() -> bool, \"");
            code.push_str(function_name);
            code.push_str("\"),");
        }
    }

    code.push(']');

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn kernel_test_mod(item: TokenStream) -> TokenStream {
    unsafe {
        CURR_MOD = item.to_string();
    }

    String::new().parse().expect("Generated invalid tokens")
}
