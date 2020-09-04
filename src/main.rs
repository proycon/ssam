extern crate clap;
extern crate rand;
extern crate rand_pcg;

use std::fs::File;
use std::io::{BufReader,BufRead,Lines,Write};
use std::mem;
use std::path::Path;
use rand::prelude::*;
use rand::seq::SliceRandom;
use rand_pcg::Pcg64;
use clap::{Arg, App};

enum SetSize {
    Absolute(usize),
    Relative(f64),
    Remainder,
}

fn main() {
    let args = App::new("ssam")
        .version("0.1")
        .author("Maarten van Gompel (proycon) <proycon@anaproy.nl>")
        .about("Ssam, short for split sampler, splits one or more input files into multiple sets using random sampling. Useful for splitting data into a training, test and development set. If multiple input files are specified, they are considered dependent and need to contain the same amount of units (e.g. lines)")
        .arg(Arg::with_name("delimiter")
            .long("delimiter")
            .short("delimiter")
            .takes_value(true)
            .help("Delimiter that seperates units. This is checker per line, set to an empty string to separate by an empty line. If this parameter remains unset entirely, each line will be a unit in its own right (the default)")
            )
        .arg(Arg::with_name("names")
            .long("names")
            .short("n")
            .takes_value(true)
            .help("Comma separated list of sample set names, example: train,test,dev. If not specified, sampled sets will be called sample1, sample2 etc... The sizes of each of the sets is specified with --sizes.")
            )
        .arg(Arg::with_name("sizes")
            .long("sizes")
            .short("s")
            .takes_value(true)
            .help("Comma seperated list of sizes of each of the sets to sample, i.e. the number of units to sample per set. If the number is a floating point value, it will be interpreted as a relative fraction. Use an asterisk (*) to represent all remaining units (can only be used once). Example: *,1000,1000. This value aligns with --names")
            .required(true)
            )
        .arg(Arg::with_name("replace")
            .long("replace")
            .short("-r")
            .help("Sample with replacement. This means a unit may be sampled into multiple sets or multiple times in the same set. The default is always to sample without replacement.")
            )
        .arg(Arg::with_name("seed")
            .long("seed")
            .short("-S")
            .takes_value(true)
            .help("Random seed, initialises the random number generator and allows pseudo-randomness and reproducibility.")
            )
        .arg(Arg::with_name("output")
            .long("output")
            .short("-o")
            .takes_value(true)
            .help("Output directory")
            )
        .arg(Arg::with_name("extension")
            .long("extension")
            .short("-e")
            .takes_value(true)
            .help("Output extension (defaults to txt)")
            )
        .arg(Arg::with_name("file")
            .help("file to load (if none are specified, input is read from stdin)")
            .multiple(true)
            .takes_value(true)
            .index(1)
         )

         .get_matches();


    let delimiter: Option<&str> = args.value_of("delimiter");
    let extension: &str = if args.is_present("extension") {
        args.value_of("delimiter").unwrap()
    } else {
        "txt"
    };

    let sizes: Vec<_> = args.value_of("sizes").unwrap().split(",").collect();
    let sizes: Vec<SetSize> = sizes.into_iter().map( |size|
        if size == "*" {
            SetSize::Remainder
        } else if size.contains(".") {
            SetSize::Relative(size.parse().expect("Expected a floating point value for size"))
        } else {
            SetSize::Absolute(size.parse().expect("Expected an integer value for size"))
        }
    ).collect();

    let mut setnames: Vec<String> = if args.is_present("names") {
        args.value_of("names").unwrap().split(",").map(|s| s.to_owned()).collect()
    } else {
        vec!()
    };

    while setnames.len() < sizes.len() {
        setnames.push(format!("set{}", setnames.len() + 1 ));
    }

    if setnames.len() > sizes.len() {
        eprintln!("Warning: you specified more set names than set sizes!");
    }

    let mut data: Vec<Vec<String>> = Vec::new();

    let mut outputprefixes: Vec<String> = Vec::new();
    if !args.is_present("file") {
        //no files specified, read from stdin instead
        data.push( parse_lines(std::io::stdin().lock().lines(), delimiter) );
        outputprefixes.push(
            if let Some(outputdir) = args.value_of("output") {
                outputdir.to_owned() + "/out"
            } else {
                "out".to_owned()
            }
        );
    } else {
        let files: Vec<_> = args.values_of("file").unwrap().collect();
        let mut outputprefix: Vec<String> = Vec::new();
        for filename in files.iter() {
            let file = File::open(filename).expect(format!("Unable to open file {}", filename).as_str());
            let reader = BufReader::new(file);
            data.push( parse_lines(reader.lines(), delimiter) );

            let file_stem = Path::new(filename).file_stem().unwrap().to_str().unwrap();
            outputprefixes.push(
                if let Some(outputdir) = args.value_of("output") {
                    outputdir.to_owned() + "/" + file_stem
                } else {
                    file_stem.to_owned()
                }
            );
        }
    }

    if data.is_empty() || data[0].is_empty() {
        eprintln!("ERROR: Data is empty");
        std::process::exit(1);
    }

    let datasize = data[0].len();

    //check data consistency
    if data.len() > 1 {
        for i in 1..data.len() {
            if data[i-1].len() != data[i].len() {
                eprintln!("ERROR: Input files are assumed dependent but do not match: file {} contains {} units, and file {} contains {}", i, data[i-1].len(), i+1, data[i].len());
                std::process::exit(1);
            }
        }
    }

    //keeps track of how many assignments has been made to the set corresponding with the index
    let mut totalsize: usize = 0;
    for size in sizes.iter() {
        totalsize += get_size(&size, datasize);
    }
    if totalsize > datasize && !args.is_present("replace") {
        eprintln!("ERROR: Sum of requested sample sizes exceeds the available data ({} vs {})", totalsize, datasize);
        std::process::exit(1);
    }

    //list of unassigned indices
    let mut unassigned: Vec<usize> = Vec::with_capacity(datasize);
    //mapping of data points to sets
    let mut assignment: Vec<Option<u8>> = Vec::with_capacity(datasize);
    for i in 0..datasize {
        unassigned.push(i);
        assignment.push(None); //assign to no set
    }

    //shuffle the list of unassigned items randomly
    let mut rng: Pcg64 = if args.is_present("seed") {
        let seed: u64 = args.value_of("seed").unwrap().parse().expect("Seed must be an integer value (64-bit)");
        Pcg64::seed_from_u64(seed)
    } else {
        Pcg64::from_rng(thread_rng()).expect("rng")
    };
    if !args.is_present("replace") {
        unassigned.shuffle(&mut rng);
    }


    let mut remainder_set: Option<u8> = None;

    //assign the data points to sets
    for (i, size) in sizes.iter().enumerate() {
        let i = i as u8;
        if let SetSize::Remainder = size {
            if remainder_set.is_some() {
                eprintln!("ERROR: You can only set one set's size to remainder (*)");
                std::process::exit(1);
            }
            remainder_set = Some(i);
        } else {
            let targetsize = get_size(&size, datasize);
            if args.is_present("replace") {
                //with replacement
                for _ in 0..targetsize {
                    let j: usize = rng.gen_range(0, datasize);
                    assignment[j] = Some(i);
                }
            } else {
                //without replacement
                for _ in 0..targetsize {
                    let j: usize = unassigned.pop().expect("unwrapping unassigned item");
                    assignment[j] = Some(i);
                }
            }
        }
    }

    if let Some(remainder_set) = remainder_set {
        for target in assignment.iter_mut() {
            if target.is_none() {
                *target = Some(remainder_set);
            }
        }
    } else if !unassigned.is_empty() {
        eprintln!("NOTICE: There are {} units not covered by any of the output sets", unassigned.len());
    }

    //output data
    if data.len() == 1 && sizes.len() == 1 {
        //there is only one output stream: use stdout
        output_to_stdout(&data[0], &assignment, delimiter);
    } else {
        //output to files
        output_to_files(&data, &assignment, &outputprefixes, &setnames, delimiter, extension);
    }



}

fn get_size(size: &SetSize, datasize: usize) -> usize {
   match size {
        SetSize::Absolute(size) => *size,
        SetSize::Relative(fraction) => (*fraction * datasize as f64).floor() as usize,
        SetSize::Remainder => 0
   }
}

///Parses lines into 'units' (by default equal to lines, but could be larger blocks)
fn parse_lines(lines: Lines<impl BufRead>, delimiter: Option<&str>) -> Vec<String> {
    let mut units: Vec<String> = Vec::new();
    let mut unit_buffer: String = String::new();
    for (i, line) in lines.enumerate() {
        let line = line.expect(format!("Error parsing line {}",i+1).as_str());
        if delimiter.is_none() {
            //every line is a unit
            units.push(line);
        } else if line == delimiter.unwrap() {
            units.push(mem::replace(&mut unit_buffer, String::new()));
        } else {
            if !unit_buffer.is_empty() {
                unit_buffer.push('\n');
            }
            unit_buffer += &line;
        }

    }
    if delimiter.is_some() {
        //add the final unit
        units.push(unit_buffer);
    }
    units
}


fn output_to_files(data: &Vec<Vec<String>>, assignment: &Vec<Option<u8>>, outputprefixes: &Vec<String>, setnames: &Vec<String>, delimiter: Option<&str>, extension: &str) {
    let mut filehandlers: Vec<(File,bool)> = Vec::new(); //the boolean is there to keep track if the file has been written to already
    for outputprefix in outputprefixes.iter() {
        for setname in setnames.iter() {
            let filename: String = outputprefix.clone().to_owned() +  "." + setname + "." + extension;
            let file = File::create(filename.as_str()).expect(format!("Unable to write file {}", filename.as_str()).as_str());
            filehandlers.push((file,false));
            eprintln!("Writing to {}", filename.as_str());
        }
    }

    for (i, data) in data.iter().enumerate() {
        let fh_offset = i * setnames.len();
        for (unit, assigned_set) in data.iter().zip(assignment.iter()) {
            if let Some(assigned_set) =  assigned_set {
                if let Some((file, written)) = filehandlers.get_mut(fh_offset + *assigned_set as usize) {
                    if delimiter.is_some() && *written {
                        file.write(delimiter.unwrap().as_bytes()).expect("writing to file");
                        file.write(b"\n").expect("writing to file");
                    }
                    file.write(unit.as_bytes()).expect("writing to file");
                    file.write(b"\n").expect("writing to file");
                    *written = true;
                } else {
                    eprintln!("ERROR: File handler not found for set {} (offset {})", assigned_set, fh_offset);
                    std::process::exit(2);
                }
            }
        }
    }
}

fn output_to_stdout(data: &Vec<String>, assignment: &Vec<Option<u8>>,  delimiter: Option<&str>) {
    let mut written = false;
    for (unit, assigned_set) in data.iter().zip(assignment.iter()) {
        if assigned_set.is_some() {
            if delimiter.is_some() && written {
                print!("{}\n", delimiter.unwrap());
            }
            print!("{}\n", unit);
            written = true;
        }
    }
}
