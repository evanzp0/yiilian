# About DHT

### 名词解释

* [Node](/routing_table/node.rs) 是 DHT 网络中的一个终端节点，有IP和port, 还有一个20字节的 hash id
* K 桶是在本地存放 node 的容器. K 桶组成的路由表，可以用于 hash id 的寻址，找到最近的 node 列表.
* [Peer](/routing_table/peer.rs)  是一个 bt 客户端的，有IP和port, 还有一个20字节的 hash id，
其中 token 是定时刷新，和另一个 bt 客户端通讯时在握手时发送给对方，对方回传会带上 token 以此判断对方是否是上次连接需要响应的客户端
一个 DHT 网络中的 Node, 会存放某个资源ID (hash 20 bit) 对应的 peer list，这些 peer 都在上传或下载该资源.

### Client
* query (在事务上记录 query_type)
    * ping
        ```json 
        { "t": "aa", "y": "q", "q": "ping", "a": {"id" : "<querying nodes id>"} }
        ```
    * find_node
        ```json
        { "t": "aa", "y": "q", "q": "find_node", "a": {"id" : "<querying nodes id>", "target" : "<id of target node>"} }
        ```
    * get_peers
        ```json 
        {"t":"aa", "y":"q", "q":"get_peers", "a": {"id" : "<querying nodes id>", "info_hash" : "<20-byte infohash of target torrent>"}}
        ```
        > info_hash : torrent 文件的 infohash, 20个字节的字符串类型
    * announce_peer
        ```json 
         { "t": "aa", "y": "q", "q": "announce_peer",
           "a": {
              "id" : "<querying nodes id>",  "implied_port": <0 or 1>,
              "info_hash" : "<20-byte infohash of target torrent>",
              "port" : <port number>,
              "token" : "<opaque token>"
           }
        }
        ```
        > port: 整型的端口号，表明 peer 在哪个端口下载
        > implied_port : 整型, 只能取 0 或 1。 0 表示使用 port 端口号，1 表示使用 socket.recv_from() 返回的端口号
        > token : 之前响应方在 reponse get_peers 中返回的 token 字符串，需要在 announce_peer 消息中回传给该响应方
* handle_response (找到对应的事务获取 query_type)
    * response ping
        ```json 
        { "t": "aa", "y": "r", "r": {"id" : "<queried nodes id>"} }
        ```
    * response find_node
        ```json 
        { "t":"aa", "y": "r", "r": {"id" : "<queried nodes id>", "nodes" : "<compact node info>"} } 
        ```
        > nodes : 是字符串类型，是以 id(20 bytes) + ip(4 bytes) + port(2 bytes) 的压缩格式进行叠加

    * response get_peers (注意 peers 和 nodes 是可以同时返回给请求方的)
        * 响应方找到 target 资源对应的 peers 并返回，**注意 values 是一个列表，列表中的每一个项是一个 ip + port 的二进制字符串**
            ```json
           {"t":"aa", "y":"r", "r": {"id" : "<queried nodes id>", "token" :"<opaque write token>", "values" : ["<peer 1 info string>", "<peer 2 info string>"]}}
            ```
            > values: 是列表类型
            > token : 是一个短的随即生成的字符串（二进制），有实效性，并在今后请求方的 annouce_peer 请求中必须要携带
        * 响应方没找到 target 对应 peers，则返回更近的 nodes，**注意 nodes 是压缩的 node_id + ip + port 二进制字符串，每一个 node 是 20 bytes，多个 node 可以串联**
            ```json
            {"t":"aa", "y":"r", "r": {"id" : "<queried nodes id>", "token" :"<opaque write token>", "nodes" : "<compact node info>"}}
            ```
            > nodes : 是字符串类型，是以 id(20 bytes) + ip(4 bytes) + port(2 bytes) 的压缩格式进行叠加
    * response announce_peer
        ```json
         { "t": "aa", "y": "r", "r": { "id" : "<queried nodes id>" } }
        ```
* handle_error

### Server
* handle_query
    * ping
    * find_node
    * get_peers
    * announce_peer
* 在处理请求时，如果产生错误，则返回 error 消息
    ```json
    { "t": "aa", "y": "e", "e": [ 201, "A Generic Error Ocurred" ] }
    ```
    > e : 是列表类型，只有两个元素，第一个是整形的错误码，第二个是字符串型的错误消息
    * 错误码
        * 201: 一般错误
        * 202: 服务错误
        * 203: 协议错误，比如不规范的包，无效的参数，或者错误的 toke
        * 204: 未知方法 
        