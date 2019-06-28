#!/usr/bin/env python3

# Takes a Jenkins log/artifact, saves a copy in a git repo, pushes, and rewrites the target URL
# to somewhere where the git repository is viewable from the outside world

# Requires requests

import os
import sys
import json
import uuid
import subprocess

import requests

OUTPUT_REPO_DIR = 'logs'
OUTPUT_URL_PREFIX = 'https://ajdlinux.github.io/snowpatch-ozlabs-logs'

test_result = json.load(sys.stdin)

if 'target_url' not in test_result or not test_result['target_url']:
    sys.exit()

# TODO: it might be nice for the output file name to be based on the patch name or the patch/series ID? this would require extra data in the supplied JSON blob
output_filename = str(uuid.uuid4()) + ".log"
output_path = os.path.join(OUTPUT_REPO_DIR, output_filename)

log = requests.get(test_result['target_url']).content

with open(output_path, 'wb') as output_file:
    output_file.write(log)

subprocess.run(['git', 'add', '.'], cwd=OUTPUT_REPO_DIR, stdout=subprocess.DEVNULL)
subprocess.run(['git', 'commit', '-sm', 'Add logs'], cwd=OUTPUT_REPO_DIR, stdout=subprocess.DEVNULL)
subprocess.run(['git', 'push'], cwd=OUTPUT_REPO_DIR, stdout=subprocess.DEVNULL)

test_result['target_url'] = OUTPUT_URL_PREFIX + '/' + output_filename

json.dump(test_result, sys.stdout)
