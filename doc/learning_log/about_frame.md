# <center>关于 Frame 的设计</center>

（从网络收到的）数据包 -> frame -> 业务对象

1. 数据包是二进制的

2. frame 是根据生成数据的协议来解析数据，数据包转成 frame 做两件事：
1）数据完整性校验，数据完整性校验是根据 protocol  layout 的规定完成的，并可能要进行解码，比如 DHT 的数据包需要先进行 bencode 解码，然后才能解析出 layout （它就是个 dict）
2）根据对应 protocol 中的数据 layout 来提取数据，并用它来填充 frame 中对应的 field。

所以 frame 中的field，如果不需要解码，那最简单的就是使用字节数组，如果需要先解码，那就是解码后的数据格式比如 BencodeData

3. frame 转换成业务对象，这需要根据业务规则进行业务有效性校验

示例：
```
frame {
  type: 'request',
  node_id: '000'
}

// node_id ：在生成frame时，只是从 layout 中提取了 node_id 值，并不会进行业务有效性校验；
// 但是在生成业务对象 Request 时，可能有各一个规则，就是 node_id 不可为 '000'，这个就是业务校验要做的事了
```
