FROM node:16-alpine

WORKDIR /app

RUN apk add --no-cache git bash jq

COPY . .

RUN yarn && \
	yarn codegen && \
	yarn prep:local && \
	yarn graph build --network hardhat
