use crate::utils::feature_gate;

feature_gate!(
    feature: "lua",
    mods: { lua, },
    uses: { lua::lua, }

);

pub(crate) mod types {
    macro_rules! primitive {
        ($($name:ident$(($type:ty))?),+) => {
            $(
                #[derive(Debug, Clone)]
                pub struct $name $((pub $type))?;
            )+
        };
    }

    primitive! {
        ScriptNull,

        ScriptString(String),

        ScriptBool(bool),

        ScriptU8(u8),
        ScriptI8(i8),

        ScriptU16(u16),
        ScriptI16(i16),

        ScriptU32(u32),
        ScriptI32(i32),

        ScriptU64(u64),
        ScriptI64(i64),

        ScriptF32(f32),
        ScriptF64(f64)
    }

    #[derive(Debug, Clone)]
    pub struct ScriptList<T: IntoIterator>(pub T);

    #[derive(Debug, Clone)]
    pub struct ScriptDictionary<T>(pub T);

    #[derive(Debug, Clone)]
    pub struct ScriptFunction<T>(pub T);

    #[derive(Debug)]
    pub struct ScriptError(Box<dyn std::error::Error>);
}

pub(crate) mod utils {
    #[derive(Debug, Clone)]
    pub struct Request;
}
