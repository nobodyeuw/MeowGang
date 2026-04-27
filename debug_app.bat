@echo off
echo Starting LOA Tracker with debug output...
echo ========================================
"C:\Users\leonn\AppData\Local\LOA Tracker\loatracker.exe" > debug_output.txt 2>&1
echo ========================================
echo App exited. Check debug_output.txt for details.
echo Press any key to view the output...
pause > nul
type debug_output.txt
echo.
echo Press any key to exit...
pause > nul
