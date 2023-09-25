use super::CompilerError;
use crate::get_accepted_state;
use crate::js_caller::*;
use crate::RegexAndDFA;
// use crate::{AllstrRegexDef, SubstrRegexDef};
use fancy_regex::Regex;
use itertools::Itertools;
use petgraph::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::{collections::HashMap, fs::File};
use thiserror::Error;

impl RegexAndDFA {
    pub fn gen_circom(
        &self,
        circom_path: &PathBuf,
        template_name: &str,
        gen_substrs: bool,
    ) -> Result<(), CompilerError> {
        let all_regex = String::new();
        let circom = gen_circom_allstr(&self.dfa_val, template_name)?;
        if gen_substrs {
            self.add_substrs_constraints(circom_path, circom)?;
        }
        Ok(())
    }

    pub fn add_substrs_constraints(
        &self,
        circom_path: &PathBuf,
        mut circom: String,
    ) -> Result<(), CompilerError> {
        let accepted_state =
            get_accepted_state(&self.dfa_val).ok_or(JsCallerError::NoAcceptedState)?;
        circom += "\n";
        circom += "\tsignal is_consecutive[msg_bytes+1][2];\n";
        circom += "\tis_consecutive[msg_bytes][1] <== 1;\n";
        circom += "\tfor (var i = 0; i < msg_bytes; i++) {\n";
        circom += &format!("\t\tis_consecutive[msg_bytes-1-i][0] <== states[num_bytes-i][{}] * (1 - is_consecutive[msg_bytes-i][1]) + is_consecutive[msg_bytes-i][1];\n",accepted_state);
        circom += "\t\tis_consecutive[msg_bytes-1-i][1] <== state_changed[msg_bytes-i].out * is_consecutive[msg_bytes-1-i][0];\n";
        circom += "\t}\n";

        let substr_defs_array = &self.substrs_defs.substr_defs_array;
        for (idx, defs) in substr_defs_array.into_iter().enumerate() {
            let num_defs = defs.len();
            circom += &format!("\tsignal is_substr{}[msg_bytes][{}];\n", idx, num_defs + 1);
            circom += &format!("\tsignal is_reveal{}[msg_bytes];\n", idx);
            circom += &format!("\tsignal output reveal{}[msg_bytes];\n", idx);
            circom += "\tfor (var i = 0; i < msg_bytes; i++) {\n";
            circom += &format!("\t\tis_substr{}[i][0] <== 0;\n", idx);
            for (j, (cur, next)) in defs.iter().enumerate() {
                circom += &format!(
                    "\t\tis_substr{}[i][{}] <== is_substr{}[i][{}] + ",
                    idx,
                    j + 1,
                    idx,
                    j
                );
                circom += &format!("states[i+1][{}] * states[i+2][{}];\n", cur, next);
                // if j != defs.len() - 1 {
                //     circom += " + ";
                // } else {
                //     circom += ";\n";
                // }
            }
            circom += &format!(
                "\t\tis_reveal{}[i] <== is_substr{}[i][{}] * is_consecutive[i][1];\n",
                idx, idx, num_defs
            );
            circom += &format!("\t\treveal{}[i] <== in[i+1] * is_reveal{}[i];\n", idx, idx);
            circom += "\t}\n";
        }
        circom += "}";
        let mut circom_file = File::create(circom_path)?;
        write!(circom_file, "{}", circom)?;
        circom_file.flush()?;
        Ok(())
    }
}