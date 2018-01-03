#![warn(unused_extern_crates)]

extern crate tr1pd;
extern crate env_logger;
extern crate nom;
extern crate colored;

use colored::Colorize;

use tr1pd::storage::BlockStorage;
use tr1pd::blocks::BlockPointer;
use tr1pd::crypto;
use tr1pd::crypto::PublicKey;
use tr1pd::cli;
use tr1pd::cli::tr1pctl::build_cli;

use std::io;
use std::env;
use std::str;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;

fn load_pubkey(pk: &str) -> Result<PublicKey, ()> {
    let mut file = File::open(pk).expect("create lt.pk");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    let pk = PublicKey::from_slice(&buf).unwrap();
    Ok(pk)
}

fn main() {
    env_logger::init().unwrap();

    let matches = build_cli()
        .get_matches();

    let mut path = env::home_dir().unwrap();
    path.push(".tr1pd/");
    let storage = BlockStorage::new(path);

    if matches.is_present("bash-completion") {
        cli::gen_completions(build_cli(), "tr1pctl");
    }

    if let Some(matches) = matches.subcommand_matches("init") {
        let force = matches.occurrences_of("force") > 0;

        let (pk, sk) = crypto::gen_keypair();

        {
            let mut file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .create_new(!force)
                            .open("/etc/tr1pd/lt.pk").expect("create lt.pk");
            file.write_all(&pk.0).unwrap();
        };

        {
            let mut file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .create_new(!force)
                            .open("/etc/tr1pd/lt.sk").expect("create lt.sk");
            file.write_all(&sk.0).unwrap();
        };
    }

    if let Some(matches) = matches.subcommand_matches("get") {
        let all = matches.occurrences_of("all") > 0;
        let parent = matches.occurrences_of("parent") > 0;

        let longterm_pk = load_pubkey("/etc/tr1pd/lt.pk").unwrap();

        let pointer = matches.value_of("block").unwrap();
        let pointer = BlockPointer::from_hex(pointer).unwrap();
        let block = storage.get(&pointer).unwrap();

        block.verify_longterm(&longterm_pk).expect("verify_longterm");

        if all {
            println!("{:?}", block);
        } else if parent {
            println!("{:x}", block.prev());
        } else if let Some(bytes) = block.msg() {
            println!("{}", str::from_utf8(bytes).unwrap());
        }
    }

    if let Some(_matches) = matches.subcommand_matches("head") {
        let head = storage.get_head().unwrap();
        // XXX: verify signature before printing this?
        println!("{:x}", head);
    }

    if let Some(matches) = matches.subcommand_matches("ls") {
        let longterm_pk = load_pubkey("/etc/tr1pd/lt.pk").unwrap();

        let backtrace = tr1pd::backtrace(&storage, matches.value_of("since"), None).unwrap();

        for pointer in backtrace.iter().rev() {
            let block = storage.get(&pointer).unwrap();

            // TODO: verify session as well
            block.verify_longterm(&longterm_pk).expect("verify_longterm");

            if let Some(bytes) = block.msg() {
                println!("{}", str::from_utf8(bytes).unwrap());
            }
        }
    }

    if let Some(matches) = matches.subcommand_matches("fsck") {
        let longterm_pk = load_pubkey("/etc/tr1pd/lt.pk").unwrap();

        let backtrace = tr1pd::backtrace(&storage, matches.value_of("since"), matches.value_of("to")).unwrap();
        let _verbose = matches.occurrences_of("verbose");
        let paranoid = matches.occurrences_of("paranoid") > 0;

        let mut session = None;

        // The first block in the --since parameter is trusted
        // If this is an init block this is non-fatal in paranoid mode
        let mut first_block = true;

        for pointer in backtrace.iter().rev() {
            print!("{:x} ... ", pointer);
            io::stdout().flush().unwrap();

            let buf = storage.get_raw(&pointer).unwrap();

            // TODO: do a 2-stage decode to avoid reencoding for verification

            use tr1pd::wire;
            use nom::IResult;
            if let IResult::Done(_, block) = wire::block(&buf) {
                let block = block.0;

                block.verify_longterm(&longterm_pk).expect("verify_longterm");

                use tr1pd::blocks::BlockType;
                match *block.inner() {
                    BlockType::Init(ref init) => {
                        print!("{}  ... ", "init".yellow());
                        io::stdout().flush().unwrap();

                        if paranoid && !first_block {
                            panic!("2nd init block is not allowed in paranoid mode");
                        }

                        session = Some(init.pubkey().clone());
                        // println!("ALERT: init: {:?}", session);
                    },
                    BlockType::Rekey(ref rekey) => {
                        print!("rekey ... ");
                        io::stdout().flush().unwrap();

                        rekey.verify_session(&session.unwrap()).expect("verify_session");

                        session = Some(rekey.pubkey().clone());
                        // println!("rekey: {:?}", session);
                    },
                    BlockType::Alert(ref alert) => {
                        print!("alert ... ");
                        io::stdout().flush().unwrap();

                        alert.verify_session(&session.unwrap()).expect("verify_session");

                        session = Some(alert.pubkey().clone());
                        // println!("alert: {:?}", session);
                    },
                    BlockType::Info(ref info) => {
                        print!("info  ... ");
                        io::stdout().flush().unwrap();

                        info.verify_session(&session.unwrap()).expect("verify_session");
                        // println!("info");
                    },
                };
            } else {
                panic!("corrupted entry");
            }

            println!("{}", "ok".green());
            first_block = false;
        }
    }
}
