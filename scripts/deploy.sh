#!/bin/bash
# /home/youruser/deploy/deploy.sh

APP_NAME=zero2prod
REPO_DIR=/home/youruser/deploy/$APP_NAME
DOCKER_IMAGE=$APP_NAME:latest

cd $REPO_DIR || exit 1

echo "拉取最新代码..."
git pull origin main

echo "构建镜像..."
docker build -t $DOCKER_IMAGE .

echo "重启容器..."
docker stop $APP_NAME || true
docker rm $APP_NAME || true
docker run -d --name $APP_NAME -p 1202:1202 $DOCKER_IMAGE
