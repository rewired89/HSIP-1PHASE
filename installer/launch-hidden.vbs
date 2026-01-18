' HSIP Silent Launcher - Starts processes with no visible windows
' Usage: wscript launch-hidden.vbs "path\to\program.exe" "arguments"

Set objShell = CreateObject("WScript.Shell")
Set objArgs = WScript.Arguments

If objArgs.Count < 1 Then
    WScript.Quit 1
End If

' Get program path and arguments
strProgram = objArgs(0)
strArgs = ""
If objArgs.Count > 1 Then
    strArgs = objArgs(1)
End If

' Launch hidden (0 = hide window, False = don't wait)
objShell.Run """" & strProgram & """ " & strArgs, 0, False

Set objShell = Nothing
