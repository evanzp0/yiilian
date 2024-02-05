# BT 规范文档及参考资料介绍

1. BT 基础协议

    (1) [BEP0003 (The BitTorrent Protocol Specification)](http://www.bittorrent.org/beps/bep_0003.html)

        BT 协议的作者 Bram Cohen 编写, 最具权威性, 是标准的概览.

    (2) [Incentives Build Robustness in BitTorrent](http://www.bittorrent.org/bittorrentecon.pdf)

        BT 协议的作者 Bram Cohen 编写, 其中 piece selection 和 peer selection 部分十分详细.

    > *以上资料缺点是有些部分不是很详细*

    (3) [BitTorrent Protocol -- BTP/1.0](./res/BitTorrent%20Protocol--BTP_1.0.pdf)

        这个是 DIKU 这个大学做的一份标准，写地蛮详细的，可以作为 (1)、(2) 到 (4) 的过渡阅读
    
    (4) [Bittorrent Protocol Specification v1.0](https://wiki.theory.org/BitTorrentSpecification) 

        由 BT development community维护第一版的规范，最为详细 (没梯子可以看 res 目录中的本地文件)
    
    (5) [BEP0052 (The BitTorrent Protocol Specification v2)](http://bittorrent.org/beps/bep_0052.html)

        由 BT development community维护第二版的规范，其中主要是对 SHA1 被破解的应对措施（将 SHA256 截断为 20 字节来向下兼容），
        还有对 P2P 协议延迟的优化和对文件规范进行合理改进。

    (6) [Understanding of BitTorrent Protocol](./res/Understanding%20of%20BitTorrent%20Protocol.pdf)

        由 kevinkoo001 写的一份介绍 bt 协议的 ppt 可以在和 (3) 一起看

2. BT 协议扩展

    BT 扩展规范官方列表在这里：<http://www.bittorrent.org/beps/bep_0000.html>, 其中比较重要的扩展如下：

    - Peer Wire 相关

        (1) [BEP0006 (Fast Extension)](http://www.bittorrent.org/beps/bep_0006.html)
        
            扩展了 Message 消息里面的关键字，而其中最主要的就是 Allowed Fast 关键字，其功能就是让新加入的 peer 能快速获得若干 piece 然后愉快地下载。

        (2) [BEP0009 (Extension for Peers to Send Metadata Files)]

            此扩展的目的是允许客户端通过资源的 info-hash 加入群，下载 metadata 信息到内存，而无需先下载 .torrent 文件。

        (3) [BEP0010 (Extension Protocol)](http://www.bittorrent.org/beps/bep_0010.html)
            
            此扩展的目的是为 BitTorrent 协议的扩展提供简单而精简的传输。 支持此协议可以轻松添加新扩展，而不会干扰 BitTorrent 标准协议，也不会影响不支持此扩展的客户端。

        (4) [BEP0011 (Peer Exchange)](http://www.bittorrent.org/beps/bep_0011.html)
            
            实现此扩展的 peer 在收到握手消息后，会将自己的 DHT 端口使用 port 消息发送给对方，PEX 为群体提供另一种对等点发现机制。

            PEX 跟 DHT 最大的区别就是，不需要向特定的 nodes 获得 peer 信息，只需要自己任意的有 connection 的 peer（当然首先都要支持PEX扩展）相互交换就行。
            需要指出的是 DHT 是单独的 extension 而 PEX 是建立在之前介绍的 BEP0010 (Extension Protocol) 之上的。

        (5) [BEP0012 (Multitracker Metadata Extension)](http://www.bittorrent.org/beps/bep_0012.html)

            这个支持 annoucelist 的，作为单 tracker 的替代，作用有二: backup 和 loadbalance 。
            backup 是指几个 tracker 之间信息不能共享的，loadbalance 是信息能共享的，加上这两种目的的混合形式，annoucelist 有三种形式。

        (6) [BEP0018 (Search Engine Specification)](http://www.bittorrent.org/beps/bep_0018.html)

            为搜索提供便利。
            下面是我google这个功能找到的一个解释，在：http://file.org/extension/btsearch
            The .btsearch extension is used by the BitTorrent peer-to-peer file sharing applications. 
            The BTSEARCH files control how the BitTorrent client searches for a torrent on a particular P2P search engine.
            This allows the search engines, such as Google, to be added to a user's built-in torrent search bar. 
            The BTSEARCH files contain the name, URL and description of the search engine that is being added.

        (7) [BEP0019 (WebSeed - HTTP/FTP Seeding)](http://www.bittorrent.org/beps/bep_0019.html)

            更详细的 web seed，它对于 piece selection 的 rarest first 方案有修改。
            key idea 是尽量让 p2p 来传输小的连续的没有的 piece，又叫 gap，大的 gap 就给 HTTP/FTP 来传输。
            所以修改后的 piece selection 会选 gap 小的 piece，除非稀有程度相当巨大。

        (8) [BEP0021 (Extension for partial seeds)](http://www.bittorrent.org/beps/bep_0021.html)

            partial seed 指那些没有完整下载 torrent 里面的文件，但又不需要继续下载的情况。
            比如，torrent 里面有多个文件而 user 只想要下载其中的一部分。
            更细地区分 partial seed 与其他的 incomplete 可以给与 client 更为准确的健康度信息。

        (9) [BEP0027 (Private Torrents)](http://www.bittorrent.org/beps/bep_0027.html)

            这个扩展目的是是某一些 torrent 分享局限在一定的用户群体之中，对于断开再连接以及 PEX 是否支持都有特别要求，以防止非指定的用户对于 private torrent 的访问。


        (10) [BEP0029 (uTorrent transport protocol)](http://www.bittorrent.org/beps/bep_0029.html)

            正常的 peer 之间实际的 file share 采用的是 TCP 协议。由于 TCP 建立大量的连接会造成其他高优先级的网络服务（比如 email、phone call、brower WEB）延迟。
            采用了一种架设在 UDP 之上的新的传输层协议，采用了基于延迟的拥塞控制，可以在没有其他要求的时候充分利用带宽，在有上述服务的时候让出带宽。

        (11) [BEP0030 (Merkle hash torrent extension)](http://www.bittorrent.org/beps/bep_030.html)

            随着BT要传输的文件越来越大，为了使 torrent 文件保持一定的大小，piece 便越来越大（我看了一下17G的星际穿越每个 piece 已经达到了8MB），
            但是太大的 piece 又会给 peer之间的 piece 交换带来不利影响，一个方面是最开始要等很久才能得到一个完整的 piece 以开始和 peer 的正常交换。
            此扩展采用了一种 hash tree 的结构，将每一个 piece 的 hash 作为树叶，每一个父节点又是子节点的 hash，这样总会得到一个 root 的 hash。
            验证的时候，通过下载的 piece 计算 hash 再与传输的其父节点的 hash 再计算hash这样会计算出一个 root hash 对比以验证 piece 完整性。
            这样在 torrent 文件之中只需要存放 root hash 可以保持 torrent 文件很小，但是也可以将每个 piece 选地较小。

    - Tracker 相关

        (1) [BEP0007 (IPv6 Tracker Extension)](http://www.bittorrent.org/beps/bep_0007.html)

            Tracker 的 IPv6的扩展

        (2) [BEP0015 (UDP Tracker Protocol for BitTorrent)](http://www.bittorrent.org/beps/bep_0015.html)

            采用 UDP 来代替在 TCP 上的 TCP 在 client 和 tracker 之间的信息传输。目的：
            1. reduce traffic 50%
            2. reduce the complexity of tracker code
            本来的 HTTP GET Request 以及 Response 变为 connect request -- connect response -- announce request -- announce response

        (3) [BEP0022 (BitTorrent Local Tracker Discovery Protocol)](http://www.bittorrent.org/beps/bep_0022.html)

            这是为了方便一些 ISP，它们会部署一些 Cache 在自己的网中，自己的用户向这些 Cache 变成的 peer 下载东西的过程中，Cache 的 upload 带宽没有限制，
            并且 Cache 可以有很大的 storage 存放很多东西，这样将流量控制在一定的范围之内，节约了运营商的带宽。
            此协议就是帮助找到这些运营商部署的 Cache peer 的。

        (4) [BEP0023 (Trackers Return Compact Peer Lists)](http://www.bittorrent.org/beps/bep_0023.html)

            tracker 应答的最重要内容当然就是peer信息了。BEP3中规定的返回格式是 a list of dicts，一个dict就表示一个peer信息，dict{peerid，ip，port}
            而通过此扩展，可以返回a string of multi-6bytes， 4 bytes for ip，2 bytes for port。
            此扩展的主要目的就是节约带宽。

        (5) [BEP0024 (Tracker Returns External IP)](http://www.bittorrent.org/beps/bep_0024.html)
            
            所有的节点和tracker交换信息，在 tracker 一端看到的 client 的 IP 都是它们的公网 IP（当然如果 client 和 tracker 在同一个私网的话就还是私网 IP）。
            client 收到的 Tracker 的 Response 里面的 peer 的 IP 信息都是公网 IP，部署这个协议的目的是为了从 tracker 那里获得自己的公网IP。

        (6) [BEP0026 (Zeroconf Peer Advertising and Discovery)](http://www.bittorrent.org/beps/bep_0026.html)

            这个扩展就是使用一系列的 zeroconf 技术实现在局域网中寻找 peer，从而最大化 BT 的效用。

        (7) [BEP0028 (Tracker exchange extension)](http://www.bittorrent.org/beps/bep_0028.html)

            这个明显是和 PEX 相对的交换 tracker 信息的扩展，可以简称 TEX，交换那些自身 verify 了的 tracker 给 peer。

        (8) [BEP0048 (Tracker Protocol Extension: Scrape)](http://www.bittorrent.org/beps/bep_0048.html) 

            peer 向 tracker 发送 scrape 请求，只查询当前种子的人数，没有开始停止等操作，不会把自己的ip公布到tracker列表中，也不会获得完整的peer列表占用服务器宽带。
            也可以用于检测 Tracker 的状态，方便告知用户或进行调试。

    - DHT 相关
    
        (1) [BEP0005 (DHT Protocol)](http://www.bittorrent.org/beps/bep_0005.html) , [学习笔记](./bep_005/learning_log.md)
            
            非常重要的 extension，实现了真正意义上的分布式系统，之前的协议从 tracker 的意义上来说仍然是集中式的，tracker 仍然存在单点故障以及瓶颈链路问题。

        (2) [BEP0032 (IPv6 extension for DHT)](http://bittorrent.org/beps/bep_0032.html)

            该 DHT 扩展，增加了对 IPv6 的支持。

        (3) [BEP0033 (DHT scrape)](http://bittorrent.org/beps/bep_0033.html)

            该扩展是通过基于布隆过滤器的分布式计数，来支持对 DHT 的 scrape 查询请求。
        
        (4) [BEP0042 (DHT Security Extension)](http://bittorrent.org/beps/bep_0042.html)

            通过 Node ID 的校验算法调整，使得对 DHT 发起一些特定攻击变得更加困难，同时也让网络窥探更加困难。

        (5) [BEP0043 (Read-only DHT Nodes)](http://bittorrent.org/beps/bep_0043.html)

            此扩展引入了“只读”DHT 节点的概念，适用于位于限制性 NAT 后面的设备，其穿透已失败使得该节点本质上无法联系，
            也适用于额外的网络流量可能会影响用户设备或经济的情况（比如当设备具有流量限制，或流量对电池寿命产生不利影响等等）。

        (6) [BEP0044 (Storing arbitrary data in the DHT)](http://bittorrent.org/beps/bep_0044.html)

            此扩展允许在 DHT中存储和检索任意数据，它支持存储不可变条目和可变条目。

        (7) [BEP0045 (Multiple-address operation for the BitTorrent DHT)](http://bittorrent.org/beps/bep_0045.html)

            此 BEP 为希望在主机（具有多个全局可路由单播地址）上操作 DHT 节点的客户端提供实施建议。

            单播（unicast）是指数据包在计算机网络的传输中，目的地址为单一目标的一种传输方式。通常所使用的网络协议大多采用单播传输，例如TCP和UDP。
            除单播传输方式外，还有广播（broadcast）和多播（multicast）。它们与单播的区别是，广播的目的地址为网络中的全体目标，而多播的目的地址是一组目标，加入该组的成员均是数据包的目的地。

        (8) [BEP0046 (Updating Torrents Via DHT Mutable Items)](http://bittorrent.org/beps/bep_0046.html)

            此扩展使 torrent 能够根据存储在 DHT（而不是 HTTP 服务器）中的数据进行更新。 发布者可以通过可变的 DHT 项来控制 torrent 的更新时间及其包含的内容。 
            它类似于 BEP 39，不同之处在于它使用 DHT 来通知和接收有关 torrent 更新的通知，因此应该为发布者和消费者提供更去中心化的功能。

        (9) [BEP0051 (DHT Infohash Indexing)](http://bittorrent.org/beps/bep_0051.html)

            此扩展使 DHT 节点能够发送 sample_infohashes 请求，检索其他节点当前存储中的 infohash 样本。