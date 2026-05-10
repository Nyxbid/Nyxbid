#!/bin/bash
set -eux
dnf update -y
dnf install -y docker git
systemctl enable --now docker
usermod -aG docker ec2-user
curl -SL https://github.com/docker/compose/releases/latest/download/docker-compose-linux-x86_64 \
  -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose
ln -sf /usr/local/bin/docker-compose /usr/bin/docker-compose

cd /home/ec2-user
sudo -u ec2-user git clone https://github.com/Nyxbid/Nyxbid.git nyxbid
chown -R ec2-user:ec2-user /home/ec2-user/nyxbid
