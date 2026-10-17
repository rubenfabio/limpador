# Shows a Windows notification. Title and message come from LIMPADOR_TITULO and
# LIMPADOR_MENSAGEM (environment), so no text needs to be escaped on the command line.
$titulo = $env:LIMPADOR_TITULO
$mensagem = $env:LIMPADOR_MENSAGEM

[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
$template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent(
    [Windows.UI.Notifications.ToastTemplateType]::ToastText02)
$textos = $template.GetElementsByTagName("text")
$textos.Item(0).AppendChild($template.CreateTextNode($titulo)) | Out-Null
$textos.Item(1).AppendChild($template.CreateTextNode($mensagem)) | Out-Null

# Notifications need a registered app id; Windows PowerShell's is always present.
$appId = '{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe'
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($appId).Show(
    [Windows.UI.Notifications.ToastNotification]::new($template))
