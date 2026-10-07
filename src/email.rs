use serde_json::json;

use crate::config::Config;

fn new_ticket_email_html(app_url: &str, ticket_id: uuid::Uuid, subject: &str, body: &str) -> String {
    format!(
        r#"<div style="background-color:#F4F6F5; padding:40px 20px; font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:480px; margin:0 auto;">
    <tr>
      <td style="background-color:#FFFFFF; border:1px solid #E4E8E6; border-radius:14px; padding:32px; text-align:left;">

        <table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 0 20px;">
          <tr>
            <td style="width:36px; height:36px; border-radius:9px; background:linear-gradient(135deg,#3C7A69,#2C5A4D);">
              <table role="presentation" width="100%" height="36" cellpadding="0" cellspacing="0">
                <tr><td align="center" valign="middle" style="color:#FFFFFF; font-size:16px;">🎫</td></tr>
              </table>
            </td>
          </tr>
        </table>

        <p style="margin:0 0 4px; font-size:12px; font-weight:700; color:#3C7A69; text-transform:uppercase; letter-spacing:.04em;">
          New ticket
        </p>
        <h1 style="margin:0 0 16px; font-size:19px; font-weight:800; color:#1A2420; letter-spacing:-0.2px; line-height:1.3;">
          {subject}
        </h1>

        <div style="background-color:#F7F9F8; border:1px solid #E4E8E6; border-radius:10px; padding:16px 18px; margin-bottom:20px;">
          <p style="margin:0; font-size:14px; line-height:1.65; color:#3A4640; white-space:pre-wrap;">{body}</p>
        </div>

        <a href="{app_url}/dashboard"
           style="display:inline-block; background-color:#3C7A69; color:#FFFFFF; font-size:14px; font-weight:700; text-decoration:none; padding:11px 24px; border-radius:8px;">
          Open in dashboard
        </a>

        <p style="margin:20px 0 0; font-size:11.5px; color:#9AA6A0; font-family:monospace;">
          #{ticket_id}
        </p>

      </td>
    </tr>
    <tr>
      <td style="text-align:center; padding-top:20px;">
        <p style="margin:0; font-size:11px; color:#A9B3AE;">Anoline</p>
      </td>
    </tr>
  </table>
</div>"#
    )
}

fn reply_email_html(app_url: &str, subject: &str, body: &str, portal: bool) -> String {
    let button = if portal {
        format!(
            r#"<a href="{app_url}/portal" style="display:inline-block; background-color:#3C7A69; color:#FFFFFF; font-size:14px; font-weight:700; text-decoration:none; padding:11px 24px; border-radius:8px;">View ticket</a>"#
        )
    } else {
        String::new()
    };

    format!(
        r#"<div style="background-color:#F4F6F5; padding:40px 20px; font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:480px; margin:0 auto;">
    <tr>
      <td style="background-color:#FFFFFF; border:1px solid #E4E8E6; border-radius:14px; padding:32px; text-align:left;">

        <table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 0 20px;">
          <tr>
            <td style="width:36px; height:36px; border-radius:9px; background:linear-gradient(135deg,#3C7A69,#2C5A4D);">
              <table role="presentation" width="100%" height="36" cellpadding="0" cellspacing="0">
                <tr><td align="center" valign="middle" style="color:#FFFFFF; font-size:16px;">💬</td></tr>
              </table>
            </td>
          </tr>
        </table>

        <p style="margin:0 0 4px; font-size:12px; font-weight:700; color:#3C7A69; text-transform:uppercase; letter-spacing:.04em;">
          New reply
        </p>
        <h1 style="margin:0 0 16px; font-size:19px; font-weight:800; color:#1A2420; letter-spacing:-0.2px; line-height:1.3;">
          {subject}
        </h1>

        <div style="background-color:#F7F9F8; border:1px solid #E4E8E6; border-radius:10px; padding:16px 18px; margin-bottom:20px;">
          <p style="margin:0; font-size:14px; line-height:1.65; color:#3A4640; white-space:pre-wrap;">{body}</p>
        </div>

        {button}

        <p style="margin:20px 0 0; font-size:11.5px; color:#9AA6A0;">
          Reply to this email to continue the conversation.
        </p>

      </td>
    </tr>
    <tr>
      <td style="text-align:center; padding-top:20px;">
        <p style="margin:0; font-size:11px; color:#A9B3AE;">Anoline</p>
      </td>
    </tr>
  </table>
</div>"#
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn send_new_ticket_email(
    config: &Config,
    agent_emails: Vec<String>,
    ticket_id: uuid::Uuid,
    subject: &str,
    body: &str,
) {
    if agent_emails.is_empty() {
        return;
    }

    let config = config.clone();
    let subject = subject.to_string();
    let body = body.to_string();

    tokio::spawn(async move {
        let html = new_ticket_email_html(
            &config.app_url,
            ticket_id,
            &escape_html(&subject),
            &escape_html(&body),
        );
        for to in agent_emails {
            if let Err(e) = send(&config, &to, &format!("New ticket: {subject}"), &html, None).await {
                tracing::error!(error = ?e, "failed to send new-ticket email");
            }
        }
    });
}

pub fn send_reply_email(
    config: &Config,
    to: &str,
    ticket_id: uuid::Uuid,
    subject: &str,
    body: &str,
    message_id: String,
    in_reply_to: Option<String>,
    portal: bool,
) {
    let config = config.clone();
    let to = to.to_string();
    let subject = subject.to_string();
    let body = body.to_string();

    tokio::spawn(async move {
        let clean = subject.trim_start_matches("Re: ").trim_start_matches("RE: ");
        let short = ticket_id.simple().to_string()[..8].to_string();
        let html = reply_email_html(&config.app_url, &escape_html(clean), &escape_html(&body), portal);

        let mut headers = json!({ "Message-ID": message_id });
        if let Some(prev) = &in_reply_to {
            headers["In-Reply-To"] = json!(prev);
            headers["References"] = json!(prev);
        }

        let subj = format!("Re: {clean} [#{short}]");
        if let Err(e) = send(&config, &to, &subj, &html, Some(headers)).await {
            tracing::error!(error = ?e, "failed to send reply email");
        }
    });
}

async fn send(
    config: &Config,
    to: &str,
    subject: &str,
    html: &str,
    headers: Option<serde_json::Value>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let mut payload = json!({
        "from": config.from_email,
        "to": to,
        "subject": subject,
        "html": html,
    });
    if let Some(h) = headers {
        payload["headers"] = h;
    }

    let res = client
        .post("https://api.resend.com/emails")
        .bearer_auth(&config.resend_api_key)
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        anyhow::bail!("resend returned {}: {}", res.status(), res.text().await.unwrap_or_default());
    }

    Ok(())
}