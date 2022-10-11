use crate::{cli::CliCommand, errors::CliError};
use clap::ArgMatches;
use clockwork_client::{queue::objects::Trigger, webhook::objects::HttpMethod};
use clockwork_utils::{AccountMetaData, InstructionData};
use serde::{Deserialize as JsonDeserialize, Serialize as JsonSerialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};
use std::{convert::TryFrom, fs, path::PathBuf, str::FromStr};

impl TryFrom<&ArgMatches> for CliCommand {
    type Error = CliError;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        match matches.subcommand() {
            Some(("api", matches)) => parse_api_command(matches),
            Some(("config", matches)) => parse_config_command(matches),
            Some(("delegation", matches)) => parse_delegation_command(matches),
            Some(("initialize", matches)) => parse_initialize_command(matches),
            Some(("localnet", matches)) => parse_bpf_command(matches),
            Some(("pool", matches)) => parse_pool_command(matches),
            Some(("queue", matches)) => parse_queue_command(matches),
            Some(("registry", _matches)) => Ok(CliCommand::RegistryGet {}),
            Some(("webhook", matches)) => parse_webhook_command(matches),
            Some(("worker", matches)) => parse_worker_command(matches),
            _ => Err(CliError::CommandNotRecognized(
                matches.subcommand().unwrap().0.into(),
            )),
        }
    }
}

// Command parsers
fn parse_bpf_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    let mut program_infos = Vec::<ProgramInfo>::new();

    if let Some(values) = matches.values_of("bpf_program") {
        let values: Vec<&str> = values.collect::<Vec<_>>();
        for address_program in values.chunks(2) {
            match address_program {
                [address, program] => {
                    let address = address
                        .parse::<Pubkey>()
                        .or_else(|_| read_keypair_file(address).map(|keypair| keypair.pubkey()));

                    if address.is_err() {
                        return Err(CliError::InvalidAddress);
                    }

                    let program_path = PathBuf::from(program);

                    if !program_path.exists() {
                        return Err(CliError::InvalidProgramFile);
                    }

                    program_infos.push(ProgramInfo {
                        program_id: address.unwrap(),
                        program_path,
                    });
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(CliCommand::Localnet { program_infos })
}

fn parse_api_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    match matches.subcommand() {
        Some(("new", matches)) => Ok(CliCommand::ApiNew {
            ack_authority: parse_pubkey("ack_authority", matches)?,
            base_url: parse_string("base_url", matches)?,
        }),
        _ => Err(CliError::CommandNotRecognized(
            matches.subcommand().unwrap().0.into(),
        )),
    }
}

fn parse_config_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    match matches.subcommand() {
        Some(("get", _)) => Ok(CliCommand::ConfigGet {}),
        _ => Err(CliError::CommandNotRecognized(
            matches.subcommand().unwrap().0.into(),
        )),
    }
}

fn parse_delegation_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    match matches.subcommand() {
        Some(("create", matches)) => Ok(CliCommand::DelegationCreate {
            worker_id: parse_u64("worker_id", matches)?,
        }),
        Some(("deposit", matches)) => Ok(CliCommand::DelegationDeposit {
            amount: parse_u64("amount", matches)?,
            delegation_id: parse_u64("delegation_id", matches)?,
            worker_id: parse_u64("worker_id", matches)?,
        }),
        Some(("get", matches)) => Ok(CliCommand::DelegationGet {
            delegation_id: parse_u64("delegation_id", matches)?,
            worker_id: parse_u64("worker_id", matches)?,
        }),
        _ => Err(CliError::CommandNotRecognized(
            matches.subcommand().unwrap().0.into(),
        )),
    }
}

fn parse_initialize_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    Ok(CliCommand::Initialize {
        mint: parse_pubkey("mint", matches)?,
    })
}

fn parse_webhook_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    Ok(CliCommand::WebhookRequestNew {
        api: parse_pubkey("api", matches)?,
        id: parse_string("id", matches)?,
        method: parse_http_method("method", matches)?,
        route: parse_string("route", matches)?,
    })
}

fn parse_worker_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    match matches.subcommand() {
        Some(("create", matches)) => Ok(CliCommand::WorkerCreate {
            signatory: parse_keypair_file("signatory_keypair", matches)?,
        }),
        Some(("get", matches)) => Ok(CliCommand::WorkerGet {
            id: parse_u64("id", matches)?,
        }),
        _ => Err(CliError::CommandNotRecognized(
            matches.subcommand().unwrap().0.into(),
        )),
    }
}

fn parse_pool_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    match matches.subcommand() {
        Some(("get", matches)) => Ok(CliCommand::PoolGet {
            id: parse_u64("id", matches)?,
        }),
        Some(("list", _)) => Ok(CliCommand::PoolList {}),
        _ => Err(CliError::CommandNotRecognized(
            matches.subcommand().unwrap().0.into(),
        )),
    }
}

fn parse_queue_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
    match matches.subcommand() {
        Some(("create", matches)) => Ok(CliCommand::QueueCreate {
            id: parse_string("id", matches)?,
            kickoff_instruction: parse_instruction_file("kickoff_instruction", matches)?,
            trigger: parse_trigger(matches)?,
        }),
        Some(("get", matches)) => Ok(CliCommand::QueueGet {
            id: parse_string("id", matches)?,
        }),
        Some(("update", matches)) => Ok(CliCommand::QueueUpdate {
            id: parse_string("id", matches)?,
            rate_limit: parse_u64("rate_limit", matches).map_or(None, |v| Some(v)),
        }),
        _ => Err(CliError::CommandNotRecognized(
            matches.subcommand().unwrap().0.into(),
        )),
    }
}

// fn parse_snapshot_command(matches: &ArgMatches) -> Result<CliCommand, CliError> {
//     Ok(CliCommand::SnapshotGet {
//         entry_id: match matches.subcommand() {
//             Some(("entry", matches)) => Some(parse_u64("id", matches)?),
//             _ => None,
//         },
//     })
// }

// Arg parsers

fn parse_trigger(matches: &ArgMatches) -> Result<Trigger, CliError> {
    if matches.is_present("account") {
        return Ok(Trigger::Account {
            pubkey: parse_pubkey("address", matches)?,
        });
    } else if matches.is_present("cron") {
        return Ok(Trigger::Cron {
            schedule: parse_string("cron", matches)?,
            skippable: true,
        });
    } else if matches.is_present("immediate") {
        return Ok(Trigger::Immediate);
    }

    Err(CliError::BadParameter("trigger".into()))
}

fn parse_instruction_file(arg: &str, matches: &ArgMatches) -> Result<InstructionData, CliError> {
    let filepath = parse_string(arg, matches)?;
    let text = fs::read_to_string(filepath).map_err(|_err| CliError::BadParameter(arg.into()))?;
    let ix: JsonInstructionData =
        serde_json::from_str(text.as_str()).expect("JSON was not well-formatted");
    InstructionData::try_from(&ix)
}

fn parse_keypair_file(arg: &str, matches: &ArgMatches) -> Result<Keypair, CliError> {
    Ok(read_keypair_file(parse_string(arg, matches)?)
        .map_err(|_err| CliError::BadParameter(arg.into()))?)
}

fn parse_http_method(arg: &str, matches: &ArgMatches) -> Result<HttpMethod, CliError> {
    Ok(HttpMethod::from_str(parse_string(arg, matches)?.as_str())
        .map_err(|_err| CliError::BadParameter(arg.into()))?)
}

fn parse_pubkey(arg: &str, matches: &ArgMatches) -> Result<Pubkey, CliError> {
    Ok(Pubkey::from_str(parse_string(arg, matches)?.as_str())
        .map_err(|_err| CliError::BadParameter(arg.into()))?)
}

fn parse_string(arg: &str, matches: &ArgMatches) -> Result<String, CliError> {
    Ok(matches
        .value_of(arg)
        .ok_or(CliError::BadParameter(arg.into()))?
        .to_string())
}

pub fn _parse_i64(arg: &str, matches: &ArgMatches) -> Result<i64, CliError> {
    Ok(parse_string(arg, matches)?
        .parse::<i64>()
        .map_err(|_err| CliError::BadParameter(arg.into()))
        .unwrap())
}

pub fn parse_u64(arg: &str, matches: &ArgMatches) -> Result<u64, CliError> {
    Ok(parse_string(arg, matches)?
        .parse::<u64>()
        .map_err(|_err| CliError::BadParameter(arg.into()))
        .unwrap())
}

// Json parsers

#[derive(Debug, JsonDeserialize, JsonSerialize)]
pub struct JsonInstructionData {
    pub program_id: String,
    pub accounts: Vec<JsonAccountMetaData>,
    pub data: Vec<u8>,
}

impl TryFrom<&JsonInstructionData> for InstructionData {
    type Error = CliError;

    fn try_from(value: &JsonInstructionData) -> Result<Self, Self::Error> {
        Ok(InstructionData {
            program_id: Pubkey::from_str(value.program_id.as_str())
                .map_err(|_err| CliError::BadParameter("Could not parse pubkey".into()))?,
            accounts: value
                .accounts
                .iter()
                .map(|ix| AccountMetaData::try_from(ix).unwrap())
                .collect::<Vec<AccountMetaData>>(),
            data: value.data.clone(),
        })
    }
}

// pub fn _parse_instruction(filepath: &String) -> Result<Instruction, CliError> {
//     let text =
//         fs::read_to_string(filepath).map_err(|_err| CliError::BadParameter("filepath".into()))?;
//     let ix: JsonInstructionData =
//         serde_json::from_str(text.as_str()).expect("JSON was not well-formatted");
//     Instruction::try_from(&ix)
// }

#[derive(Debug, JsonDeserialize, JsonSerialize, PartialEq)]
pub struct JsonAccountMetaData {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl TryFrom<&JsonAccountMetaData> for AccountMetaData {
    type Error = CliError;

    fn try_from(value: &JsonAccountMetaData) -> Result<Self, Self::Error> {
        Ok(AccountMetaData {
            pubkey: Pubkey::from_str(value.pubkey.as_str())
                .map_err(|_err| CliError::BadParameter("Could not parse pubkey".into()))?,
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProgramInfo {
    pub program_id: Pubkey,
    pub program_path: PathBuf,
}
