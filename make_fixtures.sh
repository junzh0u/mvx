#!/bin/bash

mkfile -v 1g large

for i in {0..4}; do
    cpx -f large fixtures/large$i
    cpx -f large fixtures/large_dir/file$i
done

trash -v large
