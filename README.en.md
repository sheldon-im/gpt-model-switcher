# GPT Model Switcher

English | [한국어](README.md)

I made this app out of curiosity: could switching models and reasoning levels to suit different tasks be a little easier?

Everyday tasks and demanding work such as coding call for different models and reasoning levels. But out of habit—or because switching is inconvenient or we simply forget—we often keep using the same model. I wanted to make that a little easier to change.

## Install and try it

- [Mac 0.1.5 downloads](https://github.com/sheldon-im/gpt-model-switcher/releases/tag/v0.1.5): Apple Silicon Mac, macOS 13 or later
- [Windows 0.1.5 test release](https://github.com/sheldon-im/gpt-model-switcher/releases/tag/v0.1.5-windows): Windows 11 x64

### Windows

1. Download and run `gpt-model-switcher-0.1.5-windows-x64-test-setup.exe`. This test version is not code-signed, so Windows may show a security confirmation.
2. Launch **GPT Model Switcher** after installation. If an older version is running, first choose **종료 / Quit** from its notification-area menu.
3. For portable use, extract the entire `gpt-model-switcher-0.1.5-windows-x64-test.zip` archive and run `gpt-model-switcher.exe`. Keep the accompanying `nvda` folder; it contains the files needed for NVDA announcements.
4. Closing the settings window leaves the app running in the notification area. Press Win+B, use the arrow keys to find the app, then press Shift+F10 to open its menu and choose **설정 / Settings** or **종료 / Quit**. If the icon is hidden, open the hidden-icons menu first.

### Mac

1. Download and open `gpt-model-switcher-0.1.5-macos-arm64-test.dmg`. If you prefer ZIP, download and extract `gpt-model-switcher-0.1.5-macos-arm64.zip` instead.
2. Copy **GPT Model Switcher** to Applications. If an older version is running, first choose **종료 / Quit** from the menu bar.
3. Launch **GPT Model Switcher** from Applications. It runs in the menu bar without opening the settings window automatically.
4. When prompted, allow **GPT Model Switcher** in **System Settings → Privacy & Security → Accessibility**. You can also use the app's **Open Accessibility Settings** button. With VoiceOver, find the app's switch and press VO+Space to turn it on.

This version has not been notarized by Apple, so macOS may show a security confirmation when you first open it. You may need to grant Accessibility permission again after replacing the app with a newer version.

## Switch models

Open an existing conversation in ChatGPT. Wait until its response has finished, place focus in the message input, then press a shortcut:

| Preset | Model | Reasoning level | Windows shortcut | Mac shortcut |
| --- | --- | --- | --- | --- |
| Everyday | GPT-6.1 Terra | medium | Ctrl+Alt+Shift+1 | Command+Option+Shift+1 |
| Coding | GPT-6.1 Sol | high | Ctrl+Alt+Shift+2 | Command+Option+Shift+2 |

The preset names are suggestions. In Settings, you can change the name, model, reasoning level, and shortcut to any supported combination. Choose a model available to your account.

If the same combination is already selected, the app announces the current state without changing it again. It stops if a response is being generated or it cannot identify the target conversation. Support for new conversations, cloud conversations, and remote conversations is limited.

## Change settings

On Windows, open the app's notification-area menu. On Mac, open its menu-bar menu. Choose **GPT Model Switcher → 설정 / Settings**. Use Tab and Shift+Tab to move between controls, then choose **Save** when finished.

- Edit your model, reasoning level, and shortcut combinations. The model field takes a model ID such as `gpt-6.1-terra`.
- Turn visual notifications and screen-reader announcements on or off separately. Visual notifications are off by default on Mac.
- Use **Test announcement** to hear feedback without changing the model. Save any announcement-setting changes before testing.
- Use **Recent results** to review switches from the current app session.

Closing the settings window leaves the app running. To exit completely, choose **종료 / Quit** from the Windows notification area or the Mac menu bar. The app does not register itself to start automatically at login.
