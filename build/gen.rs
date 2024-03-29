extern crate codegen;
use crate::protocol::{BlackmagicCameraProtocol, Parameter};
use codegen::{Block, Scope};
use convert_case::{Case, Casing};

static DERIVE_TRAITS: [&str; 6] = [
    "Debug",
    "PartialEq",
    "Clone",
    "PartialOrd",
    "Serialize",
    "Deserialize",
];

pub struct Datagen {
    protocol: BlackmagicCameraProtocol,
}

impl Datagen {
    pub fn new(protocol: BlackmagicCameraProtocol) -> Datagen {
        return Datagen { protocol };
    }

    pub fn gen_command(&mut self) -> String {
        let mut scope = Scope::new();

        Datagen::imports(&mut scope);
        Datagen::commands(&mut scope, &self.protocol);
        Datagen::parameters(&mut scope, &self.protocol);

        scope.to_string()
    }

    fn imports(s: &mut Scope) {
        s.import(
            "crate::rawcommand",
            "{CommandError, RawCommand, ParamType, Parameter}",
        );
        s.import("std", "fmt");
        s.import("serde", "{Serialize, Deserialize}");
    }

    fn commands(s: &mut Scope, protocol: &BlackmagicCameraProtocol) {
        let data = s.new_enum("Command").vis("pub");
        for t in DERIVE_TRAITS {
            data.derive(t);
        }

        data.allow("dead_code");

        for category in protocol.groups.iter() {
            data.new_variant(&category.normalized_name.to_case(Case::UpperCamel))
                .tuple(&category.normalized_name.to_case(Case::UpperCamel));
        }

        let im = s.new_impl("Command");

        // from_raw func
        {
            let from_raw = im
                .new_fn("from_raw")
                .arg("data", "&[u8]")
                .ret("Result<Self, CommandError>")
                .vis("pub");

            from_raw.line("let raw_cmd = RawCommand::from_raw(data)?;");
            from_raw.line("match raw_cmd.category");

            let mut match_block = Block::new("");
            for category in protocol.groups.iter() {
                match_block.line(format!(
                    "{} => Ok(Command::{1}({1}::from_raw(raw_cmd)?)),",
                    &category.id,
                    &category.normalized_name.to_case(Case::UpperCamel)
                ));
            }
            match_block.line("_ => Err(CommandError::CategoryNotDefined)");
            from_raw.push_block(match_block);
        }

        // id func
        {
            let id_func = im.new_fn("id").arg_ref_self().ret("u8").vis("pub");

            id_func.line("match self");

            let mut match_block = Block::new("");
            for category in protocol.groups.iter() {
                match_block.line(format!(
                    "Command::{}(_) => {},",
                    &category.normalized_name.to_case(Case::UpperCamel),
                    &category.id
                ));
            }
            id_func.push_block(match_block);
        }

        // parameter func
        {
            let parameter_func = im
                .new_fn("parameter_id")
                .arg_ref_self()
                .ret("u8")
                .vis("pub");

            parameter_func.line("match self");

            let mut match_block = Block::new("");
            for category in protocol.groups.iter() {
                match_block.line(format!(
                    "Command::{}(v) => v.id(),",
                    &category.normalized_name.to_case(Case::UpperCamel),
                ));
            }
            parameter_func.push_block(match_block);
        }

        //raw_type func
        {
            let am = im.new_fn("raw_type").arg_ref_self().ret("u8").vis("pub");
            am.line("match self");

            let mut mb = Block::new("");

            for category in protocol.groups.iter() {
                mb.line(format!(
                    "Command::{}(v) => v.raw_type(),",
                    &category.normalized_name.to_case(Case::UpperCamel),
                ));
            }
            am.push_block(mb);
        }

        //to_bytes func
        {
            let am = im
                .new_fn("to_bytes")
                .arg_ref_self()
                .ret("Vec<u8>")
                .vis("pub");
            am.line("match self");

            let mut mb = Block::new("");

            for category in protocol.groups.iter() {
                mb.line(format!(
                    "Command::{}(v) => v.to_bytes(),",
                    &category.normalized_name.to_case(Case::UpperCamel),
                ));
            }
            am.push_block(mb);
        }

        //name func
        {
            let am = im
                .new_fn("normalized_name")
                .arg_ref_self()
                .ret("(String, String)")
                .vis("pub");
            am.line("match self");

            let mut mb = Block::new("");

            for category in protocol.groups.iter() {
                mb.line(format!(
                    "Command::{}(v) =>  (\"{}\".to_string(), v.normalized_name()),",
                    &category.normalized_name.to_case(Case::UpperCamel),
                    &category.normalized_name,
                ));
            }
            am.push_block(mb);
        }

        //Display trait
        {
            let tr = s
                .new_impl("Command")
                .impl_trait("fmt::Display")
                .new_fn("fmt")
                .arg_ref_self()
                .arg("f", "&mut fmt::Formatter<'_>")
                .ret("fmt::Result")
                .line("match self");

            let mut mb = Block::new("");

            for category in protocol.groups.iter() {
                mb.line(format!(
                    "Command::{}(v) =>  write!(f, \"{{}}\", v.to_string()),",
                    &category.normalized_name.to_case(Case::UpperCamel)
                ));
            }

            tr.push_block(mb);
        }
    }

    fn parameters(s: &mut Scope, protocol: &BlackmagicCameraProtocol) {
        //Enums
        for category in protocol.groups.iter() {
            let data = s
                .new_enum(&category.normalized_name.to_case(Case::UpperCamel))
                .vis("pub");
            for t in DERIVE_TRAITS {
                data.derive(t);
            }

            data.allow("dead_code");

            for param in category.parameters.iter() {
                let t = lookuptype(param);
                if t != "Void" {
                    data.new_variant(&param.normalized_parameter.to_case(Case::UpperCamel))
                        .tuple(t);
                } else {
                    data.new_variant(&param.normalized_parameter.to_case(Case::UpperCamel));
                }
            }
        }

        //Implementations of trait
        for category in protocol.groups.iter() {
            let im = s
                .new_impl(&category.normalized_name.to_case(Case::UpperCamel))
                .impl_trait("Parameter");

            //id function
            {
                let id_block = im.new_fn("id").arg_ref_self().ret("u8");

                id_block.line("match self");

                let mut enum_block = codegen::Block::new("");
                for param in category.parameters.iter() {
                    if lookuptype(&param) == "Void" {
                        enum_block.line(format!(
                            "{}::{} => {},",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                            &param.id
                        ));
                    } else {
                        enum_block.line(format!(
                            "{}::{}(_) => {},",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                            &param.id
                        ));
                    }
                }
                id_block.push_block(enum_block);
            }

            //from_raw function
            {
                let raw_block = im
                    .new_fn("from_raw")
                    .arg("cmd", "RawCommand")
                    .ret("Result<Self, CommandError>");

                raw_block.line("match cmd.parameter");

                let mut match_block = Block::new("");

                for param in category.parameters.iter() {
                    if lookuptype(&param) != "Void" {
                        match_block.line(format!(
                            "{} => Ok({}::{}(ParamType::from_bytes(&cmd.data)?)),",
                            &param.id,
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                        ));
                    } else {
                        match_block.line(format!(
                            "{} => Ok({}::{}),",
                            &param.id,
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                        ));
                    }
                }

                match_block.line("_ => Err(CommandError::ParameterNotDefined),");

                raw_block.push_block(match_block);
            }

            //Raw type
            {
                let am = im.new_fn("raw_type").arg_ref_self().ret("u8");
                am.line("match self");

                let mut mb = Block::new("");

                for param in category.parameters.iter() {
                    if lookuptype(&param) != "Void" {
                        mb.line(format!(
                            "{}::{}(_) => {},",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                            typeid(&param)
                        ));
                    } else {
                        mb.line(format!(
                            "{}::{} => {},",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                            typeid(&param)
                        ));
                    }
                }
                am.push_block(mb);
            }

            //to_bytes
            {
                let am = im.new_fn("to_bytes").arg_ref_self().ret("Vec<u8>");
                am.line("match self");

                let mut mb = Block::new("");

                for param in category.parameters.iter() {
                    if lookuptype(&param) != "Void" {
                        mb.line(format!(
                            "{}::{}(v) => v.to_bytes(),",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                        ));
                    } else {
                        mb.line(format!(
                            "{}::{} => vec![0],",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                        ));
                    }
                }
                am.push_block(mb);
            }

            //normalized_name
            {
                let am = im.new_fn("normalized_name").arg_ref_self().ret("String");
                am.line("match self");

                let mut mb = Block::new("");

                for param in category.parameters.iter() {
                    if lookuptype(&param) != "Void" {
                        mb.line(format!(
                            "{}::{}(_) => \"{}\".to_string(),",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                            &param.normalized_parameter
                        ));
                    } else {
                        mb.line(format!(
                            "{}::{} => \"{}\".to_string(),",
                            &category.normalized_name.to_case(Case::UpperCamel),
                            &param.normalized_parameter.to_case(Case::UpperCamel),
                            &param.normalized_parameter
                        ));
                    }
                }
                am.push_block(mb);
            }
        }

        //Implementations of fmt::Display for subenum
        for category in protocol.groups.iter() {
            let im = s
                .new_impl(&category.normalized_name.to_case(Case::UpperCamel))
                .impl_trait("fmt::Display")
                .new_fn("fmt")
                .arg_ref_self()
                .arg("f", "&mut fmt::Formatter<'_>")
                .ret("fmt::Result")
                .line("match self");
            let mut mb = Block::new("");

            for param in category.parameters.iter() {
                if lookuptype(&param) != "Void" {
                    mb.line(format!(
                        "{}::{}(v) => write!(f, \"{{}}\", v.data_as_string()),",
                        &category.normalized_name.to_case(Case::UpperCamel),
                        &param.normalized_parameter.to_case(Case::UpperCamel),
                    ));
                } else {
                    mb.line(format!(
                        "{}::{} => write!(f, \"true\"),",
                        &category.normalized_name.to_case(Case::UpperCamel),
                        &param.normalized_parameter.to_case(Case::UpperCamel),
                    ));
                }
            }
            im.push_block(mb);
        }
    }
}

fn lookuptype(p: &Parameter) -> &'static str {
    if p.index.len() > 0 {
        match p.type_field.as_ref() {
            "void" => "Void",
            "int8" => "Vec<i8>",
            "int16" => "Vec<i16>",
            "int32" => "Vec<i32>",
            "int64" => "Vec<i64>",
            "string" => "String",
            "fixed16" => "Vec<f32>",
            _ => "Void",
        }
    } else {
        match p.type_field.as_ref() {
            "void" => "Void",
            "int8" => "i8",
            "int16" => "i16",
            "int32" => "i32",
            "int64" => "i64",
            "string" => "String",
            "fixed16" => "f32",
            _ => "Void",
        }
    }
}

fn typeid(p: &Parameter) -> u8 {
    match p.type_field.as_ref() {
        "void" => 0,
        "int8" => 1,
        "int16" => 2,
        "int32" => 3,
        "int64" => 4,
        "string" => 5,
        "fixed16" => 128,
        _ => 0,
    }
}
