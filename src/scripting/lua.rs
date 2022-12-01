use std::error::Error;

use mlua::prelude::*;

use super::{types::*, utils::*};

/// Quickly implement the `FromLua` trait.
macro_rules! impl_from_lua {
    ($primitive_type:ty {() $block:block}) => {
        impl<'lua> FromLua<'lua> for $primitive_type {
            fn from_lua(_: mlua::Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
                $block
            }
        }
    };

    ($primitive_type:ty {($v:ident) $block:block}) => {
        impl<'lua> FromLua<'lua> for $primitive_type {
            fn from_lua($v: mlua::Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
                $block
            }
        }
    };

    ($primitive_type:ty {($l:ident) $block:block}) => {
        impl<'lua> FromLua<'lua> for $primitive_type {
            fn from_lua(_: mlua::Value<'lua>, $l: &'lua Lua) -> mlua::Result<Self> {
                $block
            }
        }
    };

    ($primitive_type:ty {($v:ident, $l:ident) $block:block}) => {
        impl<'lua> FromLua<'lua> for $primitive_type {
            fn from_lua($v: mlua::Value<'lua>, $l: &'lua Lua) -> mlua::Result<Self> {
                $block
            }
        }
    };
}

/// Repeat the `impl_from_lua!` macro.
macro_rules! multi_impl_from_lua {
    (
        $($type:ty {
            $m:pat => $r:expr
        })+
    ) => {
        $(
            impl_from_lua!(
                $type {
                    (lua_value) {
                        match lua_value {
                            $m => $r,
                            _ => Err(LuaError::UserDataTypeMismatch),
                        }
                    }
                }
            );
        )+
    };
}

multi_impl_from_lua!(
    ScriptNull {
        LuaValue::Nil => Ok(ScriptNull)
    }

    ScriptString {
        LuaValue::String(s) => Ok(ScriptString(s.to_str()?.to_string()))
    }

    ScriptBool {
        LuaValue::Boolean(b) => Ok(ScriptBool(b))
    }

    ScriptU8 {
        LuaValue::Integer(i) => Ok(ScriptU8(i as u8))
    }
    ScriptI8 {
        LuaValue::Integer(i) => Ok(ScriptI8(i as i8))
    }
    ScriptU16 {
        LuaValue::Integer(i) => Ok(ScriptU16(i as u16))
    }
    ScriptI16 {
        LuaValue::Integer(i) => Ok(ScriptI16(i as i16))
    }
    ScriptU32 {
        LuaValue::Integer(i) => Ok(ScriptU32(i as u32))
    }
    ScriptI32 {
        LuaValue::Integer(i) => Ok(ScriptI32(i))
    }
    ScriptU64 {
        LuaValue::Integer(i) => Ok(ScriptU64(i as u64))
    }
    ScriptI64 {
        LuaValue::Integer(i) => Ok(ScriptI64(i as i64))
    }

    ScriptF32 {
        LuaValue::Number(f) => Ok(ScriptF32(f as f32))
    }
    ScriptF64 {
        LuaValue::Number(f) => Ok(ScriptF64(f))
    }
);

impl mlua::UserData for Request {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        #[cfg(feature = "reqwest")]
        methods.add_method::<_, ScriptString, _, _>("get", |_, _, url| {
            let res = match reqwest::blocking::get(url.0) {
                Ok(a) => a.text().unwrap_or_default(),
                Err(e) => format!("{}", e),
            };

            Ok(res)
        });
    }
}

pub fn lua() -> Result<Lua, Box<dyn Error>> {
    let lua = Lua::new();
    {
        let globals = lua.globals();

        #[cfg(feature = "reqwest")]
        {
            let request_constructor = lua.create_function(|_, ()| Ok(Request))?;
            globals.set("reqwest", request_constructor)?;
        }
    }

    Ok(lua)
}
