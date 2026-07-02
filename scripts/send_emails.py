#!/usr/bin/env python3
"""
发送邮件到潜在客户（4家）

使用方法:
  python3 scripts/send_emails.py
  然后输入邮箱密码
"""

import smtplib, ssl, os, sys, base64
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from email.mime.base import MIMEBase
from email import encoders

SMTP_SERVER = "mails.wedevs.org"
SMTP_PORT = 465
SMTP_USER = "jerry"
SENDER = "jerry@mails.wedevs.org"
PDF_PATH = "/home/de/works/auto-mining-system/docs/product-intro.pdf"

# ─── 4家客户 ─────────────────────────────────────
EMAILS = [
    {
        "to": "hnnyjtzhaopin@163.com",
        "subject": "矿山运输调度系统方案 - 王先生推荐",
        "body_file": "/tmp/email_body_0.txt",
        "note": "湖南能源集团"
    },
    {
        "to": "252486942@qq.com",
        "subject": "矿山运输调度系统方案 - 王先生",
        "body_file": "/tmp/email_body_1.txt",
        "note": "锦江集团（三门峡锦江矿业）"
    },
    {
        "to": "yjsnw@nem-cn.com",
        "subject": "新建矿山运输调度系统方案建议",
        "body_file": "/tmp/email_body_2.txt",
        "note": "斯诺威矿业（宁德时代）"
    },
    {
        "to": "1543264111@qq.com",
        "subject": "矿山智能调度系统 - 产品介绍",
        "body_file": "/tmp/email_body_3.txt",
        "note": "贵州天弘矿业"
    },
]


def send_email(to: str, subject: str, body: str, attachment: str, password: str):
    msg = MIMEMultipart()
    msg["From"] = SENDER
    msg["To"] = to
    msg["Subject"] = subject

    msg.attach(MIMEText(body, "plain", "utf-8"))
    msg["Subject"] = ("=?UTF-8?B?" + base64.b64encode(subject.encode()).decode() + "?=")

    with open(attachment, "rb") as f:
        part = MIMEBase("application", "octet-stream")
        part.set_payload(f.read())
        encoders.encode_base64(part)
        part.add_header(
            "Content-Disposition",
            f'attachment; filename="{os.path.basename(attachment)}"',
        )
        msg.attach(part)

    context = ssl.create_default_context()
    with smtplib.SMTP_SSL(SMTP_SERVER, SMTP_PORT, context=context) as server:
        server.login(SMTP_USER, password)
        server.sendmail(SENDER, [to], msg.as_string())


def main():
    if not os.path.exists(PDF_PATH):
        print(f"❌ 附件不存在: {PDF_PATH}")
        sys.exit(1)

    print("=" * 50)
    print("📧 发送 4 封邮件")
    print("=" * 50)
    for i, e in enumerate(EMAILS):
        to = e["to"] or "（未填写）"
        print(f"{i+1}. {e['note']:12s} → {to}")
    print()

    # 跳过未填写邮箱的
    valid = [(i, e) for i, e in enumerate(EMAILS) if e["to"]]
    if not valid:
        print("❌ 没有可发送的邮箱")
        sys.exit(1)

    password = os.environ.get("EMAIL_PASS")
    if not password:
        print("用法: EMAIL_PASS='你的密码' python3 scripts/send_emails.py")
        sys.exit(1)

    success = 0
    for idx, email in valid:
        print(f"\n📨 发送 ({idx+1}/{len(EMAILS)}) {email['note']}...", end=" ")
        sys.stdout.flush()

        with open(email["body_file"]) as f:
            body = f.read()

        try:
            send_email(email["to"], email["subject"], body, PDF_PATH, password)
            print("✅")
            success += 1
        except Exception as e:
            print(f"❌ {e}")

    print(f"\n✅ 完成: {success}/{len(valid)} 封发送成功")

    print("\n💡 锦江集团的邮件发到了三门峡锦江矿业（252486942@qq.com）")
    print("   如果这个不对，也可以在猎聘上直接联系他们的招聘HR")


if __name__ == "__main__":
    main()
