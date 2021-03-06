# Type Conversions

The type `Value` represents Lisp values:
- They can be copied around, but cannot outlive the `Env` they come from.
- They are "proxy values": only useful when converted to Rust values, or used as arguments when calling Lisp functions.

## Converting a Lisp `Value` to Rust

This is enabled for types that implement `FromLisp`. Most built-in types are supported. Note that conversion may fail, so the return type is `Result<T>`.

```rust
let i: i64 = value.into_rust()?; // error if Lisp value is not an integer
let f: f64 = value.into_rust()?; // error if Lisp value is nil

let s = value.into_rust::<String>()?;
let s: Option<&str> = value.into_rust()?; // None if Lisp value is nil
```

## Converting a Rust Value to Lisp

This is enabled for types that implement `IntoLisp`. Most built-in types are supported. Note that conversion may fail, so the return type is `Result<Value<'_>>`.

```rust
"abc".into_lisp(env)?;
"a\0bc".into_lisp(env)?; // NulError (Lisp string cannot contain null byte)

5.into_lisp(env)?;
65.3.into_lisp(env)?;

().into_lisp(env)?; // nil
true.into_lisp(env)?; // t
false.into_lisp(env)?; // nil
```
