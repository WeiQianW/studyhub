Mechatronics Study Hub — Windows app
=====================================

1. Unzip this folder anywhere (for example Documents\StudyHub). Keep both files together:
     MechatronicsStudyHub.exe
     WebView2Loader.dll
2. Double-click MechatronicsStudyHub.exe.

Works offline
  Every note, flashcard, quiz question, formula, calculator and the PLC Lab is inside the app.
  No internet connection is needed.

Updates
  Whenever the computer is online, the app checks for a newer version (at start-up and every
  15 minutes). New files are downloaded in the background, every file is checked against its
  SHA-256 fingerprint, and only then is the new version switched in. A banner then offers
  "Reload"; otherwise the update is used the next time you open the app. If you are offline,
  nothing happens and the app keeps using the version it has.

  Update address: https://weiqianw.github.io/studyhub/
  To use a different address, put it (https:// only) in a file named update-url.txt in
  %LOCALAPPDATA%\MechatronicsStudyHub

Your data
  Progress, flashcards you know, quiz best score, PLC programs and notebook highlights are kept in
  %LOCALAPPDATA%\MechatronicsStudyHub (they stay when the app updates).
  update.log in that folder shows when the app last checked for updates.

Needs Windows 10 or 11 with the Microsoft Edge WebView2 runtime (already installed on Windows 11
and on up-to-date Windows 10). If the window stays blank, install "WebView2 Runtime" from
Microsoft's website.

Windows may show "Windows protected your PC" the first time because the app is not code-signed:
choose "More info" → "Run anyway".
