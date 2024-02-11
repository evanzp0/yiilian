# <center>关于 Frame 的设计</center>

（从网络收到的）数据包 -> frame -> 业务对象

1. 数据包是二进制的

2. frame 是根据生成数据的协议来解析数据，数据包转成 frame 只做一件事情：
数据完整性校验，数据完整性校验是根据 protocol  layout 的规定完成的。
也就是说，frame 中业务指令并不会做校验，它只是关注数据包的数据是不是协议规定的格式。
比如说 dht 规范要求传输的数据是个 bencoded dict，那我们的 frame 只要保证它是一个 bencoded dict 。至于 这个 dict 中的指令和参数的有效性校验，属于更上一层的业务逻辑了，不需要在 frame 中进行。

3. frame 转换成业务对象，这需要根据业务规则进行业务有效性校验

示例：
```
frame {
  q: 'get_peers',
  info: '000'
}

// 在生成frame时，并不会对 q 和 info 的值，进行业务有效性校验；
// 但是在生成业务对象 Query 时，可能有要校验 info 的长度必须是 20 ， get_peers 还需要 node_id 参数等等，这个就是业务校验要做的事了
```
