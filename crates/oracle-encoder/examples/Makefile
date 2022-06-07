all: 01-empty-blocknums.json \
	02-register-networks-and-set-block-numbers-same-payload.json \
	03-register-networks-and-set-block-numbers.json \
	04-register-multiple-and-set-block-numbers-thrice.json \
	05-register-multiple-and-unregister.json

01-empty-blocknums.json:
	jsonnet 01-empty-blocknums.jsonnet > $@

02-register-networks-and-set-block-numbers-same-payload.json:
	jsonnet 02-register-networks-and-set-block-numbers-same-payload.jsonnet > $@

03-register-networks-and-set-block-numbers.json:
	jsonnet 03-register-networks-and-set-block-numbers.jsonnet > $@

04-register-multiple-and-set-block-numbers-thrice.json:
	jsonnet 04-register-multiple-and-set-block-numbers-thrice.jsonnet > $@

05-register-multiple-and-unregister.json:
	jsonnet 05-register-multiple-and-unregister.jsonnet > $@

.PHONY: clean
clean:
	rm -f 01-empty-blocknums.json
	rm -f 02-register-networks-and-set-block-numbers-same-payload.json
	rm -f 03-register-networks-and-set-block-numbers.json
	rm -f 04-register-multiple-and-set-block-numbers-thrice.json
	rm -f 05-register-multiple-and-unregister.json
