complete -c uu_date -s d -l date -d 'display time described by STRING, not \'now\'' -r
complete -c uu_date -s f -l file -d 'like --date; once for each line of DATEFILE' -r -F
complete -c uu_date -s I -l iso-8601 -d 'output date/time in ISO 8601 format.
FMT=\'date\' for date only (the default),
\'hours\', \'minutes\', \'seconds\', or \'ns\'
for date and time to the indicated precision.
Example: 2006-08-14T02:34:56-06:00' -r -f -a "{date	,hours	,minutes	,seconds	,ns	}"
complete -c uu_date -l rfc-3339 -d 'output date/time in RFC 3339 format.
FMT=\'date\', \'seconds\', or \'ns\'
for date and time to the indicated precision.
Example: 2006-08-14 02:34:56-06:00' -r -f -a "{date	,seconds	,ns	}"
complete -c uu_date -s r -l reference -d 'display the last modification time of FILE' -r -F
complete -c uu_date -s s -l set -d 'set time described by STRING' -r
complete -c uu_date -s R -l rfc-email -d 'output date and time in RFC 5322 format.
Example: Mon, 14 Aug 2006 02:34:56 -0600'
complete -c uu_date -l debug -d 'annotate the parsed date, and warn about questionable usage to stderr'
complete -c uu_date -s u -l universal -d 'print or set Coordinated Universal Time (UTC)'
complete -c uu_date -s h -l help -d 'Print help'
complete -c uu_date -s V -l version -d 'Print version'
