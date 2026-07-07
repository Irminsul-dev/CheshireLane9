from mitmproxy import http


def request(flow: http.HTTPFlow) -> None:
    # Redirect SDK API requests to the local Cheshire SDK server.
    if (
        flow.request.host == "jp-sdk-api.yostarplat.com"
        or flow.request.host == "en-sdk-api.yostarplat.com"
    ):
        flow.request.host = "127.0.0.1"
        flow.request.port = 21080
        flow.request.scheme = "http"
