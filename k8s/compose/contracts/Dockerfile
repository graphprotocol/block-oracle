FROM node:16-alpine

WORKDIR /app

RUN apk add --no-cache git python3 py3-pip make build-base bash jq

COPY package.json yarn.lock ./

RUN yarn install

# Clone the repository containing all network contracts.
RUN git clone https://github.com/graphprotocol/contracts.git network-contracts && \
	cd network-contracts && \
	git checkout 35aa93

RUN cd network-contracts && yarn install && yarn build

COPY . .

RUN yarn compile
