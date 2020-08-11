use jni::sys::{_jobject, jboolean, jbyteArray, jint, jlong, jstring};
use jni::JNIEnv;
use libsignal_protocol_rust::*;
use std::fmt;

#[derive(Debug)]
pub enum SignalJniError {
    Signal(SignalProtocolError),
    Jni(jni::errors::Error),
    BadJniParameter(&'static str),
    NullHandle,
    IntegerOverflow(String),
    UnexpectedPanic(std::boxed::Box<dyn std::any::Any + std::marker::Send>),
}

impl fmt::Display for SignalJniError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SignalJniError::Signal(s) => write!(f, "{}", s),
            SignalJniError::Jni(s) => write!(f, "{}", s),
            SignalJniError::NullHandle => write!(f, "null handle"),
            SignalJniError::BadJniParameter(m) => write!(f, "bad parameter type {}", m),
            SignalJniError::IntegerOverflow(m) => {
                write!(f, "integer overflow during conversion of {}", m)
            }
            SignalJniError::UnexpectedPanic(e) => match e.downcast_ref::<&'static str>() {
                Some(s) => write!(f, "unexpected panic: {}", s),
                None => write!(f, "unknown unexpected panic"),
            },
        }
    }
}

impl From<SignalProtocolError> for SignalJniError {
    fn from(e: SignalProtocolError) -> SignalJniError {
        SignalJniError::Signal(e)
    }
}

impl From<jni::errors::Error> for SignalJniError {
    fn from(e: jni::errors::Error) -> SignalJniError {
        SignalJniError::Jni(e)
    }
}

pub fn throw_error(env: &JNIEnv, error: SignalJniError) {
    let error_string = format!("{}", error);

    let exception_type = match error {
        SignalJniError::NullHandle => "java/lang/NullPointerException",
        SignalJniError::UnexpectedPanic(_) => "java/lang/AssertionError",
        SignalJniError::BadJniParameter(_) => "java/lang/AssertionError",
        SignalJniError::IntegerOverflow(_) => "java/lang/RuntimeException",

        SignalJniError::Signal(SignalProtocolError::DuplicatedMessage(_, _)) => {
            "org/signal/libsignal/DuplicatedMessageException"
        }

        SignalJniError::Signal(SignalProtocolError::InvalidPreKeyId)
        | SignalJniError::Signal(SignalProtocolError::InvalidSignedPreKeyId)
        | SignalJniError::Signal(SignalProtocolError::InvalidSenderKeyId) => {
            "org/signal/libsignal/InvalidKeyIdException"
        }

        SignalJniError::Signal(SignalProtocolError::NoKeyTypeIdentifier)
        | SignalJniError::Signal(SignalProtocolError::BadKeyType(_))
        | SignalJniError::Signal(SignalProtocolError::BadKeyLength(_, _)) => {
            "org/signal/libsignal/InvalidKeyException"
        }

        SignalJniError::Signal(SignalProtocolError::SessionNotFound) => {
            "org/signal/libsignal/NoSessionException"
        }

        SignalJniError::Signal(SignalProtocolError::InvalidMessage(_))
        | SignalJniError::Signal(SignalProtocolError::CiphertextMessageTooShort(_))
        | SignalJniError::Signal(SignalProtocolError::UnrecognizedCiphertextVersion(_))
        | SignalJniError::Signal(SignalProtocolError::UnrecognizedMessageVersion(_))
        | SignalJniError::Signal(SignalProtocolError::InvalidCiphertext)
        | SignalJniError::Signal(SignalProtocolError::InvalidProtobufEncoding) => {
            "org/signal/libsignal/InvalidMessageException"
        }

        SignalJniError::Signal(SignalProtocolError::LegacyCiphertextVersion(_)) => {
            "org/signal/libsignal/LegacyMessageException"
        }

        SignalJniError::Signal(SignalProtocolError::InvalidState(_, _))
        | SignalJniError::Signal(SignalProtocolError::NoSenderKeyState)
        | SignalJniError::Signal(SignalProtocolError::InvalidSessionStructure) => {
            "java/lang/IllegalStateException"
        }

        SignalJniError::Signal(SignalProtocolError::InvalidArgument(_)) => {
            "java/lang/IllegalArgumentException"
        }

        SignalJniError::Signal(_) => "java/lang/RuntimeException",

        SignalJniError::Jni(_) => "java/lang/RuntimeException",
    };

    let _ = env.throw_new(exception_type, error_string);
}

pub type ObjectHandle = jlong;

pub unsafe fn native_handle_cast<T>(
    handle: ObjectHandle,
) -> Result<&'static mut T, SignalJniError> {
    /*
    Should we try testing the encoded pointer for sanity here, beyond
    being null? For example verifying that lowest bits are zero,
    highest bits are zero, greater than 64K, etc?
    */
    if handle == 0 {
        return Err(SignalJniError::NullHandle);
    }

    Ok(&mut *(handle as *mut T))
}

pub unsafe fn native_handle_cast_optional<T>(
    handle: ObjectHandle,
) -> Result<Option<&'static mut T>, SignalJniError> {
    if handle == 0 {
        return Ok(None);
    }

    Ok(Some(&mut *(handle as *mut T)))
}

// A dummy value to return when we are throwing an exception
pub trait JniDummyValue {
    fn dummy_value() -> Self;
}

impl JniDummyValue for ObjectHandle {
    fn dummy_value() -> Self {
        0
    }
}

impl JniDummyValue for jint {
    fn dummy_value() -> Self {
        0 as jint
    }
}

impl JniDummyValue for *mut _jobject {
    fn dummy_value() -> Self {
        0 as jstring
    }
}

impl JniDummyValue for jboolean {
    fn dummy_value() -> Self {
        0 as jboolean
    }
}

impl JniDummyValue for () {
    fn dummy_value() -> Self {
    }
}

pub fn run_ffi_safe<F: FnOnce() -> Result<R, SignalJniError> + std::panic::UnwindSafe, R>(
    env: &JNIEnv,
    f: F,
) -> R
where
    R: JniDummyValue,
{
    match std::panic::catch_unwind(f) {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            throw_error(env, e);
            R::dummy_value()
        }
        Err(r) => {
            throw_error(env, SignalJniError::UnexpectedPanic(r));
            R::dummy_value()
        }
    }
}

pub fn box_object<T>(t: Result<T, SignalProtocolError>) -> Result<ObjectHandle, SignalJniError> {
    match t {
        Ok(t) => Ok(Box::into_raw(Box::new(t)) as ObjectHandle),
        Err(e) => Err(SignalJniError::Signal(e)),
    }
}

pub fn to_jbytearray<T: AsRef<[u8]>>(
    env: &JNIEnv,
    data: Result<T, SignalProtocolError>,
) -> Result<jbyteArray, SignalJniError> {
    let data = data?;
    let data: &[u8] = data.as_ref();
    let out = env.new_byte_array(data.len() as i32)?;
    let buf: Vec<i8> = data.iter().map(|i| *i as i8).collect();
    env.set_byte_array_region(out, 0, buf.as_slice())?;
    Ok(out)
}

pub fn jint_to_u32(v: jint) -> Result<u32, SignalJniError> {
    if v < 0 {
        return Err(SignalJniError::IntegerOverflow(format!("{} to u32", v)));
    }
    Ok(v as u32)
}

pub fn jint_to_u8(v: jint) -> Result<u8, SignalJniError> {
    if v < 0 || v > 255 {
        return Err(SignalJniError::IntegerOverflow(format!("{} to u8", v)));
    }
    Ok(v as u8)
}

pub fn jint_from_u32(value: Result<u32, SignalProtocolError>) -> Result<jint, SignalJniError> {
    match value {
        Ok(value) => {
            let result = value as jint;
            if result as u32 != value {
                return Err(SignalJniError::IntegerOverflow(format!(
                    "{} to jint",
                    value
                )));
            }
            Ok(result)
        }
        Err(e) => Err(SignalJniError::Signal(e)),
    }
}

pub fn jlong_from_u64(value: Result<u64, SignalProtocolError>) -> Result<jlong, SignalJniError> {
    match value {
        Ok(value) => {
            let result = value as jlong;
            if result as u64 != value {
                return Err(SignalJniError::IntegerOverflow(format!(
                    "{} to jlong",
                    value
                )));
            }
            Ok(result)
        }
        Err(e) => Err(SignalJniError::Signal(e)),
    }
}

#[macro_export]
macro_rules! jni_fn_deserialize {
    ( $nm:ident is $func:path ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            data: jbyteArray,
        ) -> ObjectHandle {
            run_ffi_safe(&env, || {
                let data = env.convert_byte_array(data)?;
                box_object($func(data.as_ref()))
            })
        }
    };
}

#[macro_export]
macro_rules! jni_fn_get_new_boxed_obj {
    ( $nm:ident($rt:ty) from $typ:ty, $body:expr ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            handle: ObjectHandle,
        ) -> ObjectHandle {
            run_ffi_safe(&env, || {
                let obj = native_handle_cast::<$typ>(handle)?;
                return box_object::<$rt>($body(obj));
            })
        }
    };
}

#[macro_export]
macro_rules! jni_fn_get_new_boxed_optional_obj {
    ( $nm:ident($rt:ty) from $typ:ty, $body:expr ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            handle: ObjectHandle,
        ) -> ObjectHandle {
            run_ffi_safe(&env, || {
                let obj = native_handle_cast::<$typ>(handle)?;
                let result : Option<$rt> = $body(obj)?;
                if let Some(result) = result {
                    box_object::<$rt>(Ok(result))
                } else {
                    Ok(0 as ObjectHandle)
                }
            })
        }
    };
}

#[macro_export]
macro_rules! jni_fn_get_jint {
    ( $nm:ident($typ:ty) using $body:expr ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            handle: ObjectHandle,
        ) -> jint {
            run_ffi_safe(&env, || {
                let obj = native_handle_cast::<$typ>(handle)?;
                return jint_from_u32($body(obj));
            })
        }
    };
}

#[macro_export]
macro_rules! jni_fn_get_jboolean {
    ( $nm:ident($typ:ty) using $body:expr ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            handle: ObjectHandle,
        ) -> jboolean {
            run_ffi_safe(&env, || {
                let obj = native_handle_cast::<$typ>(handle)?;
                let r : bool = $body(obj)?;
                Ok(r as jboolean)
            })
        }
    };
}

/*
Without the indirection of inner_get, rust can't deduce the Error type
if the provided lambda just returns Ok(something)
*/
#[macro_export]
macro_rules! jni_fn_get_jstring {
    ( $nm:ident($typ:ty) using $body:expr ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            handle: ObjectHandle,
        ) -> jstring {
            fn inner_get(t: &$typ) -> Result<String, SignalProtocolError> {
                $body(&t)
            }
            run_ffi_safe(&env, || {
                let obj : &mut $typ = native_handle_cast::<$typ>(handle)?;
                return Ok(env.new_string(inner_get(&obj)?)?.into_inner());
            })
        }
    };
    ( $nm:ident($typ:ty) using $func:path ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            handle: ObjectHandle,
        ) -> jstring {
            run_ffi_safe(&env, || {
                let obj : &mut $typ = native_handle_cast::<$typ>(handle)?;
                return Ok(env.new_string($func(&obj)?)?.into_inner());
            })
        }
    };
}

#[macro_export]
macro_rules! jni_fn_get_jbytearray {
    ( $nm:ident($typ:ty) using $body:expr ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(
            env: JNIEnv,
            _class: JClass,
            handle: ObjectHandle,
        ) -> jbyteArray {
            run_ffi_safe(&env, || {
                let obj = native_handle_cast::<$typ>(handle)?;
                return to_jbytearray(&env, $body(obj));
            })
        }
    };
}

#[macro_export]
macro_rules! jni_fn_destroy {
    ( $nm:ident destroys $typ:ty ) => {
        #[no_mangle]
        pub unsafe extern "system" fn $nm(_env: JNIEnv, _class: JClass, handle: ObjectHandle) {
            if handle != 0 {
                let _boxed_value = Box::from_raw(handle as *mut $typ);
            }
        }
    };
}
