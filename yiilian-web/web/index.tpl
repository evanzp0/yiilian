
<html>
    <head>
        bt search
    </head>
    <body>
        <form action="/search" method="get">
            <input name="q" type="text" value="{{value | default(value='')}}" />
            <button type="submit">搜索</button>
        </form>
    </body>
</html>