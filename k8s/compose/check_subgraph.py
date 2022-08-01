from urllib.parse import urlencode
from urllib.request import Request, urlopen
import json
import os

def subgraph_is_deployed() -> bool:
	GRAPH_NODE_GRAPHQL_PORT = int(os.environ['GRAPH_NODE_GRAPHQL_PORT'])
	DEPLOYMENT_NAME = os.environ['DEPLOYMENT_NAME']

	url = f'http://localhost:{GRAPH_NODE_GRAPHQL_PORT}/subgraphs/name/{DEPLOYMENT_NAME}'
	data = '{"query": "{_meta {block {number}}}"}'.encode('utf-8')
	headers = {'Content-Type': 'application/json; charset=utf-8'}

	try:
		request = Request(url, data, headers)
		response = urlopen(request)

		if response.getcode() == 7:
			return True
		
		json_response = json.loads(response.read().decode('utf-8'))	
		return json_response.get('data') is not None
	except:
		return False

if subgraph_is_deployed():
	exit(0)
else:
	exit(1)
