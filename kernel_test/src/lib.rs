use proc_macro::TokenStream;
use std::ops::Add;

use syn::ItemFn;

static mut TESTS: Vec<String> = Vec::new();
static mut CURR_MOD: String = String::new();

#[proc_macro_attribute]
pub fn kernel_test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let function = input.to_string();

    let input_fn = syn::parse_macro_input!(input as ItemFn);
    let test_fn = input_fn.sig.ident;
    let function_full_name = unsafe { format!("{CURR_MOD}::{test_fn}") };

    let code = format!(
        r#"
        #[cfg(feature = "run_tests")]
        pub {function}
    "#
    );

    unsafe {
        TESTS.push(function_full_name.to_string());
    }

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn all_tests(_item: TokenStream) -> TokenStream {
    let mut code = "[".to_owned();

    unsafe {
        for test in &TESTS {
            let function_name = test.split(':').last().unwrap();
            code = code.add(&format!("({test} as fn(), \"{function_name}\"),"));
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
