#!/bin/sh
# Verify the Ethiopian calendar is used in the Ethiopian locale.

# Copyright (C) 2025 Free Software Foundation, Inc.

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.

# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

# . "${srcdir=.}/tests/init.sh"; path_prepend_ ./src
# print_ver_ date
# Replacing init.sh specific commands with simple ones for standalone reproduction

# Current year in the Gregorian calendar.
current_year=$(LC_ALL=C date +%Y)

export LC_ALL=am_ET.UTF-8

if ! locale -a | grep -q "am_ET.UTF-8"; then
  echo "Ethiopian UTF-8 locale not available, skipping test"
  exit 77
fi

# 09-10 and 09-12 of the same Gregorian year are in different years in the
# Ethiopian calendar.
# Note: Using the date binary from the current directory if compiled
DATE_BIN="./target/debug/date"
if [ ! -f "$DATE_BIN" ]; then
    DATE_BIN="date" # Fallback to system date if not built
    echo "Using system date: $(which date)"
else
    echo "Using compiled date: $DATE_BIN"
fi

year_september_10=$($DATE_BIN -d $current_year-09-10 +%Y)
year_september_12=$($DATE_BIN -d $current_year-09-12 +%Y)
month_name=$($DATE_BIN -d $current_year-09-10 +%B)

echo "Current Gregorian Year: $current_year"
echo "Sept 10 Ethiopian Year ($DATE_BIN): $year_september_10"
echo "Sept 10 Month ($DATE_BIN): $month_name"
echo "Sept 12 Ethiopian Year ($DATE_BIN): $year_september_12"

SYSTEM_DATE=$(which date)
if [ -x "$SYSTEM_DATE" ]; then
    sys_year=$($SYSTEM_DATE -j -f "%Y-%m-%d" "$current_year-09-10" +%Y 2>/dev/null || $SYSTEM_DATE -d "$current_year-09-10" +%Y 2>/dev/null)
    sys_month=$($SYSTEM_DATE -j -f "%Y-%m-%d" "$current_year-09-10" +%B 2>/dev/null || $SYSTEM_DATE -d "$current_year-09-10" +%B 2>/dev/null)
    echo "System Date Year: $sys_year"
    echo "System Date Month: $sys_month"
fi

if [ "$year_september_10" = "$(($year_september_12 - 1))" ]; then
    echo "PASS: Years differ as expected"
else
    echo "FAIL: Years should differ"
    fail=1
fi

# The difference between the Gregorian year is 7 or 8 years.
if [ "$year_september_10" = "$(($current_year - 8))" ]; then
     echo "PASS: Sept 10 is -8 years"
else
     echo "FAIL: Sept 10 should be -8 years, got $(($current_year - $year_september_10)) diff"
     fail=1
fi

if [ "$year_september_12" = "$(($current_year - 7))" ]; then
    echo "PASS: Sept 12 is -7 years"
else
    echo "FAIL: Sept 12 should be -7 years, got $(($current_year - $year_september_12)) diff"
    fail=1
fi

# Check that --iso-8601 and --rfc-3339 uses the Gregorian calendar.
case $($DATE_BIN --iso-8601=hours) in $current_year-*) ;; *) echo "FAIL: ISO-8601 not Gregorian"; fail=1 ;; esac
case $($DATE_BIN --rfc-3339=date) in $current_year-*) ;; *) echo "FAIL: RFC-3339 not Gregorian"; fail=1 ;; esac

if [ "$fail" = "1" ]; then
    exit 1
fi
exit 0
