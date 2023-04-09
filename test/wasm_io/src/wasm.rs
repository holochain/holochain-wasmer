use holochain_wasmer_guest::*;

macro_rules! _s {
    ( $t:tt; $empty:expr; ) => {
        paste::item! {
            #[no_mangle]
            /// ignore the input completely and return empty data
            pub extern "C" fn [< $t:lower _input_ignored_empty_ret >](guest_ptr: usize, len: usize) -> DoubleUSize {
                // Still need to deallocate the input even if we don't use it.
                crate::allocation::__hc__deallocate_1(guest_ptr, len);
                return_ptr(
                    paste::expr! {
                        test_common::[< $t:camel Type >]::from($empty)
                    }
                )
            }

            #[no_mangle]
            /// load the input args and do nothing with it
            pub extern "C" fn [< $t:lower _input_args_empty_ret >](ptr: usize, len: usize) -> DoubleUSize {
                paste::expr! {
                    let _: test_common::[< $t:camel Type >] = match host_args(ptr, len) {
                        Ok(v) => v,
                        Err(err_ptr) => return err_ptr,
                    };
                }
                return_ptr(
                    paste::expr! {
                        test_common::[< $t:camel Type >]::from($empty)
                    }
                )
            }

            #[no_mangle]
            /// load the input args and return it
            pub extern "C" fn [< $t:lower _input_args_echo_ret >](ptr: usize, len: usize) -> DoubleUSize {
                let r: test_common::[< $t:camel Type >] = match host_args(ptr, len) {
                    Ok(v) => v,
                    Err(err_ptr) => return err_ptr,
                };
                return_ptr(r)
            }
        }
    }
}

_s!(Bytes; vec![];);
_s!(String; "".to_string(););

macro_rules! _n {
    ( $t:tt; $n:ident; $inner:expr; $empty:expr; ) => {
        paste::item! {
            #[no_mangle]
            pub extern "C" fn [< $t:lower _serialize_n >](ptr: usize, len: usize) -> DoubleUSize {
                // build it
                let $n: test_common::IntegerType = match host_args(ptr, len) {
                    Ok(v) => v,
                    Err(err_ptr) => return err_ptr,
                };
                let s = paste::expr! {
                    test_common::[< $t:camel Type >]::from($inner)
                };
                // serialize it
                let _: Vec<u8> = holochain_serialized_bytes::encode(&s).unwrap();
                // return nothing
                return_ptr(
                    paste::expr! {
                        test_common::[< $t:camel Type >]::from($empty)
                    }
                )
            }

            #[no_mangle]
            pub extern "C" fn [< $t:lower _ret_n >](ptr: usize, len: usize) -> DoubleUSize {
                // build it
                let $n: test_common::IntegerType = match host_args(ptr, len) {
                    Ok(v) => v,
                    Err(err_ptr) => return err_ptr,
                };
                let s = paste::expr! {
                    test_common::[< $t:camel Type >]::from($inner)
                };
                // return it
                return_ptr(s)
            }
        }
    }
}

_n!(Bytes; n; vec![0; u32::from(n).try_into().unwrap()]; vec![];);
_n!(String; n; ".".repeat(u32::from(n).try_into().unwrap()).to_string(); "".to_string(););
