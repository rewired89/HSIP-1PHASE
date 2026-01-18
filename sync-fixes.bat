@echo off
REM Quick sync script - pulls latest fixes from branch
echo Syncing latest HSIP fixes...
echo.

git fetch origin
git reset --hard origin/claude/fix-security-vulnerabilities-Kbv48

echo.
echo Sync complete! Files updated.
echo.
pause
