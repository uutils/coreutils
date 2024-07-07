
use builtin;
use str;

set edit:completion:arg-completer[uu_date] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_date'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_date'= {
            cand -d 'display time described by STRING, not ''now'''
            cand --date 'display time described by STRING, not ''now'''
            cand -f 'like --date; once for each line of DATEFILE'
            cand --file 'like --date; once for each line of DATEFILE'
            cand -I 'output date/time in ISO 8601 format.
FMT=''date'' for date only (the default),
''hours'', ''minutes'', ''seconds'', or ''ns''
for date and time to the indicated precision.
Example: 2006-08-14T02:34:56-06:00'
            cand --iso-8601 'output date/time in ISO 8601 format.
FMT=''date'' for date only (the default),
''hours'', ''minutes'', ''seconds'', or ''ns''
for date and time to the indicated precision.
Example: 2006-08-14T02:34:56-06:00'
            cand --rfc-3339 'output date/time in RFC 3339 format.
FMT=''date'', ''seconds'', or ''ns''
for date and time to the indicated precision.
Example: 2006-08-14 02:34:56-06:00'
            cand -r 'display the last modification time of FILE'
            cand --reference 'display the last modification time of FILE'
            cand -s 'set time described by STRING'
            cand --set 'set time described by STRING'
            cand -R 'output date and time in RFC 5322 format.
Example: Mon, 14 Aug 2006 02:34:56 -0600'
            cand --rfc-email 'output date and time in RFC 5322 format.
Example: Mon, 14 Aug 2006 02:34:56 -0600'
            cand --debug 'annotate the parsed date, and warn about questionable usage to stderr'
            cand -u 'print or set Coordinated Universal Time (UTC)'
            cand --universal 'print or set Coordinated Universal Time (UTC)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
