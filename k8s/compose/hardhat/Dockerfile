FROM node:16-alpine

WORKDIR /app

EXPOSE 8545

RUN apk add --no-cache git python3 py3-pip make build-base

COPY . .
RUN yarn install

CMD [ "yarn", "hardhat", "node", "--port", "8545" ]
