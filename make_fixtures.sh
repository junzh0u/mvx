#!/bin/bash

mkfile -v 1g large

for i in {0..9}; do
    cpx large fixtures/large$i
    cpx large fixtures/large_dir/file$i
done

trash -v large
