# Ssam

Ssam, short for split sampler, splits one or more text-based input files into multiple
sets using random sampling. This is useful for splitting data into a training, test and
development sets, or whatever sets you desire.

## Features

* Split input into multiple sets, user can specify the size of each set in either absolute numbers or a relative fraction.
* Supports both sampling without replacement (default) or with replacement.
* Defaults to line-based sampling, but a custom delimiter can be configured to sample larger blocks.
* Can handle multiple input files that will be considered **dependent**. Useful for splitting and sampling for instance parallel corpora.
* Specify a seed for the random number generator to create reproducible samples.


