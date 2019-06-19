#!/usr/bin/env python3

# A hook that takes TestResults and adds encouraging messages.

import sys
import json

test_result = json.load(sys.stdin)

SUCCESS_MSG = "Woooooooooo!"
ENCOURAGING_MSG = "We believe in you! Go respin!"

if test_result['state'] == 'success':
    msg = SUCCESS_MSG
else:
    msg = ENCOURAGING_MSG

test_result['description'] = (test_result.get('description', '') + ' ' + msg).strip()

json.dump(test_result, sys.stdout)