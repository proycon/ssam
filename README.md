[![Crate](https://img.shields.io/crates/v/ssam.svg)](https://crates.io/crates/ssam)

# Ssam

Ssam, short for split sampler, splits one or more text-based input files into multiple
sets using random sampling. This is useful for splitting data into a training, test and
development sets, or whatever sets you desire.

## Features

* Split input into multiple sets, user can specify the size of each set in either absolute numbers or a relative fraction.
* Supports both sampling without replacement (default) or with replacement.
* Defaults to line-based sampling, but a custom delimiter can be configured to sample larger blocks of variable size.
* Can handle multiple input files that will be considered **dependent**. Useful for splitting and sampling for instance parallel corpora.
* By default ordering is preserved, use ``--shuffle`` for more randomness.
* Specify a seed for the random number generator to create **reproducible samples**.

## Installation

Install it using Rust's package manager:

```
cargo install ssam
```

No cargo/rust on your system yet? Do ``sudo apt install cargo`` on Debian/ubuntu based systems, ``brew install rust`` on mac, or use [rustup](https://rustup.rs/).

## Usage

See ``ssam --help`` for extensive usage information.

Suppose you have a text file ``sentences.txt`` with one sentence per line, and you want to sample the sentences into a test, development and
train set using respectively 10% (`0.1`), 10%  (`0.1`) and the remainder (`*`) of the sentences:

```
$ ssam --sizes "0.1,0.1,*" --names "test,dev,train" sentences.txt
```

This will output three files: `sentences.train.txt`, `sentences.test.txt` and `sentences.dev.txt`. If you don't specify
any names explicitly the infix will simply be ``set1``,``set2``,``set3``, etc..

Suppose you have the same sentences in German in a file called `sätze.txt` and the sentences are aligned up nicely with
the ones in `sentences.txt` (i.e. the same line numbers correspond and contain translations). You can now make a
dependent split as follows:

```
$ ssam --shuffle --sizes "0.1,0.1,*" --names "test,dev,train" sentences.txt sätze.txt
```

The sentences will still correspond in each of the output sets. We also added ``--shuffle`` for more randomness in the
output order, as by default ssam preserves order.

Ssam can also read from stdin (provided you want to supply only one input document). If you're only doing one sample
(rather than three as shown above), then it will simply output to stdout.

Rather than using lines as units, you can specify a delimiter manually. For example, set ``--delimiter ""`` (empty
delimiter) if you want empty lines to be the delimiter, such as for instance those often used to separate paragraphs.
Alternative you can set it to an explicit marker in your input, like ``--delimiter "<utt>"`` for example.

## In Memoriam

In loving memory of our cat [Sam](https://proycon.anaproy.nl/img/photos/2020-01-04:-In-Memoriam-2009-2019.jpg), 2009-2019.
