@setLocal
@echo off

:: `test-repo-whitespace [DIR]...` v2022.01.01
:: style inspector ~ whitespace: find nonconforming files in repository

:: Copyright (C) 2016-2022 ~ Roy Ivy III <rivy.dev@gmail.com>
:: License: MIT/Apache-2.0 (see https://opensource.org/licenses/Apache-2.0 , https://opensource.org/licenses/MIT)
:: * this software is provided for free, WITHOUT ANY EXPRESS OR IMPLIED WARRANTY (see the license for details)

:: spell-checker:ignore (shell/cmd) COMSPEC ERRORLEVEL
:: spell-checker:ignore () CTYPE POSIX Tval Tvar akefile makefile makefiles multiline testdata

:config
set "_exclude_dirs_rx=(?i)[_.#]build|[.]git|[.]gpg|[.]obj|[.]vs|fixtures|node_modules|target|testdata|test-resources|vendor"
set "_exclude_files_rx=(?i)[.](cache|dll|exe|gif|gz|ico|png|zip)$"
set "_crlf_files_rx=(?i)[.](bat|cmd|csv)$"
set "_tabbed_files_rx=(?i)^^(.*[.]go|go[.](mod|sum)|(GNU)?[Mm]akefile([.].*)?)$"
:config_done

set _dp0=%~dp0.
set _nx0=%~nx0
set dirs=%*
if NOT DEFINED dirs if EXIST "%CD%\repo" ( set dirs="%CD%\repo" )
if NOT DEFINED dirs if EXIST "%_dp0%\..\.git" ( set dirs="%_dp0%\.." )
if NOT DEFINED dirs ( set "dirs=." )

set PCREGREP=pcregrep
if EXIST "%_dp0%\pcregrep.EXE" ( set "PCREGREP=%_dp0%\pcregrep.EXE" )
"%PCREGREP%" --version >NUL 2>NUL || ( echo ERR!: Missing required `pcregrep` [try `scoop install pcregrep`] 1>&2 & goto _undefined_ 2>NUL || "%COMSPEC%" /d/c exit 1 )

echo [ %dirs% ]

if /i "%LC_CTYPE%"=="posix" (set "LC_CTYPE=C") &:: `pcregrep` doesn't understand the common "POSIX", replace with the equivalent "C"

set "ERRORLEVEL="
set "ERROR="
:: 1. Test for TABs within leading whitespace (except go files makefiles)
"%PCREGREP%" -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" --exclude "%_tabbed_files_rx%" --count --files-with-matches --recursive "^\s*\t" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set ERROR=1 & echo ERROR: files found with TABs within leading whitespace [file:#lines_matched])

:: 2. Test for lines with internal TABs
"%PCREGREP%" -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" --count --files-with-matches --recursive "^.*[^\t]\t" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set ERROR=1 & echo ERROR: files found with lines containing internal TABs [file:#lines_matched])

:: 3. Test that makefiles have ONLY initial-TAB leading whitespace
"%PCREGREP%" -I --exclude-dir "%_exclude_dirs_rx%" --include "(GNU)?[Mm]akefile([.].*)?" --exclude "[.](to|y)ml$" --recursive --line-number --invert-match "^([\t]\s*\S|\S|$)" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set ERROR=1 & echo ERROR: Makefiles found with lines having non-TAB leading whitespace [file:line_number])

:: 4. Test for non-LF line endings
set "HAVE_NonLF_ERROR="
"%PCREGREP%" --buffer-size=1M -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" -NLF --files-with-matches --multiline --recursive "\r[^\n]" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set HAVE_NonLF_ERROR=1 & echo ## files found with CR line endings)
"%PCREGREP%" --buffer-size=1M -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" --exclude "%_crlf_files_rx%" -NLF --files-with-matches --multiline --recursive "\r\n" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set HAVE_NonLF_ERROR=1 & echo ## files found with CRLF line endings)
if DEFINED HAVE_NonLF_ERROR ( set ERROR=1 & echo ERROR: files found with non-LF line endings)

:: 5. Test for files without trailing newline
:: "%PCREGREP%" -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" --files-without-match --multiline --recursive "\r?[\r\n]\z" %dirs%
"%PCREGREP%" -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" --files-with-matches --multiline --recursive "\z" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set ERROR=1 & echo ERROR: files found without trailing newline)

:: 6. Test for files with lines having trailing whitespace
"%PCREGREP%" -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" --recursive --line-number "\s$" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set ERROR=1 & echo ERROR: files found with lines having trailing whitespace [file:line_number])

:: 7. Test for files with BOM
"%PCREGREP%" -I --exclude-dir "%_exclude_dirs_rx%" --exclude "%_exclude_files_rx%" --files-with-matches --multiline --recursive "\A[\xEF][\xBB][\xBF]" %dirs%
if NOT "%ERRORLEVEL%" == "1" ( set ERROR=1 & echo ERROR: files found with leading BOM)

:script_done
if NOT DEFINED ERROR (
    echo success: no file errors found
    )
call :#exit__title __exit_title "%COMSPEC%"
set "__exit_title=" && title %__exit_title% && set "ERRORLEVEL=" & goto _undefined_ 2>NUL || "%COMSPEC%" /d/c "exit %ERROR%"

@rem::::
@rem:: FUNCTIONS
@goto :EOF

::
:#exit__title ( ref_RETURN PATH )
:: RETURN == name of PATH
setLocal
set "_RETvar=%~1"
set "_RETval=%~2"
if NOT DEFINED _RETval ( set "_RETval=cmd" & goto :#exit__title_RETURN )
set "_RETval=%~n2"
:#exit__title_RETURN
endLocal & set %_RETvar%^=%_RETval%
goto :EOF
::
