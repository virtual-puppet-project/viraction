use std::error::Error;

use mlua::prelude::*;

use super::{types::*, utils::*};

impl<'lua> FromLua<'lua> for ScriptString {
    fn from_lua(lua_value: mlua::Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
        match lua_value {
            LuaValue::String(s) => Ok(ScriptString(s.to_str()?.to_string())),
            _ => Err(LuaError::UserDataTypeMismatch),
        }
    }
}

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
