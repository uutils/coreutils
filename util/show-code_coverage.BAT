@setLocal
@echo off

@rem:: # spell-checker:ignore (shell/CMD) COMSPEC ERRORLEVEL

set "ME_dir=%~dp0."
set "REPO_main_dir=%ME_dir%\.."

set "ERRORLEVEL="
set "COVERAGE_REPORT_DIR=%REPO_main_dir%\target\debug\coverage-win"

call "%ME_dir%\build-code_coverage.BAT"
if ERRORLEVEL 1 goto _undefined_ 2>NUL || @for %%G in ("%COMSPEC%") do @title %%nG & @"%COMSPEC%" /d/c exit %ERRORLEVEL%

call start "" "%COVERAGE_REPORT_DIR%"\index.html
if ERRORLEVEL 1 goto _undefined_ 2>NUL || @for %%G in ("%COMSPEC%") do @title %%nG & @"%COMSPEC%" /d/c exit %ERRORLEVEL%
