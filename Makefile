export TEST_URL=http://ovh.net/files/1Mio.dat
export LOGIN=guest
export PASSWORD=guest
export HOST=127.0.0.1
export PORT=5672
export MANAGEMENT_PORT=15672
export JOB_QUEUE=doner.job.queue
export DOWNLOAD_DIR="downloads"
define CONFIG_TEMPLATE
downloader:
    threads: 5
    dns_threads: 5
    save_path: ${DOWNLOAD_DIR}
queues:
    job: ${JOB_QUEUE}
    pending: doner.pending.queue
amqp_remote:
    host: ${HOST}
    port: ${PORT}
    login: ${LOGIN}
    password: ${PASSWORD}
amqp_local:
    host: ${HOST}
    port: ${PORT}
    login: ${LOGIN}
    password: ${PASSWORD}
http:
    prefix: http://127.0.0.1:8080/
    host: 0.0.0.0
    port: 8080
    threads: 1
tls:
    ca_path: server.crt
endef
export CONFIG_TEMPLATE
prepare:
	openssl req -subj '/CN=domain.com/O=My Company Name LTD./C=US' -new -newkey rsa:2048 -days 365 -nodes -x509 -keyout server.key -out server.crt
	echo "$${CONFIG_TEMPLATE}" > config.yaml
	mkdir -p ${DOWNLOAD_DIR}
start:
	docker run -d --rm -p 5672:5672 -p 15672:15672 --name doner_test "rabbitmq:3.6-management"
pub:
	curl 'http://${LOGIN}:${PASSWORD}@${HOST}:${MANAGEMENT_PORT}/api/exchanges/%2F/amq.default/publish' \
		--data-binary '{"vhost":"/","name":"amq.default","properties":{"delivery_mode":1,"headers":{}},"routing_key":"${JOB_QUEUE}","delivery_mode":"1","payload":"{\"url\": \"${TEST_URL}\",\"reply_to\": {\"endpoint\": \"url\",\"target\":\"http://localhost:8000/from_downloader\"}}","headers":{},"props":{},"payload_encoding":"string"}' --compressed

pubq:
	curl 'http://${LOGIN}:${PASSWORD}@${HOST}:${MANAGEMENT_PORT}/api/exchanges/%2F/amq.default/publish' \
		--data-binary '{"vhost":"/","name":"amq.default","properties":{"delivery_mode":1,"headers":{}},"routing_key":"${JOB_QUEUE}","delivery_mode":"1","payload":"{\"url\": \"${TEST_URL}\",\"reply_to\": {\"endpoint\": \"queue\",\"target\":\"client.queue\"}}","headers":{},"props":{},"payload_encoding":"string"}' --compressed

pubs:
	curl 'http://${LOGIN}:${PASSWORD}@${HOST}:${MANAGEMENT_PORT}/api/exchanges/%2F/amq.default/publish' \
		--data-binary '{"vhost":"/","name":"amq.default","properties":{"delivery_mode":1,"headers":{}},"routing_key":"${JOB_QUEUE}","delivery_mode":"1","payload":"{\"url\": \"${TEST_URL}\",\"reply_to\": {\"endpoint\": \"url\",\"target\":\"http://127.0.0.1/downloader/callback\"}}","headers":{},"props":{},"payload_encoding":"string"}' --compressed
 
err:
	curl 'http://${USER}:${PASSWORD}@${HOST}:${MANAGEMENT_PORT}/api/exchanges/%2F/amq.default/publish' \
			--data-binary '{"vhost":"/","name":"amq.default","properties":{"delivery_mode":1,"headers":{}},"routing_key":"${JOB_QUEUE}","delivery_mode":"1","payload":"{\"foo\"}","headers":{},"props":{},"payload_encoding":"string"}' --compressed

stop:
	docker stop doner_test
