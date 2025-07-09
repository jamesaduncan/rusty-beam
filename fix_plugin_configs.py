import os
import re
from pathlib import Path

# Fix selector-handler config
selector_config = """<\!DOCTYPE html>
<html lang="en">
<head>
    <title>Selector Handler Plugin Test Configuration</title>
</head>
<body>
    <h1>Selector Handler Plugin Test Configuration</h1>
    
    <table itemref="host-localhost" itemscope itemtype="http://rustybeam.net/ServerConfig">
        <tbody>
            <tr>
                <td>Server Root</td>
                <td itemprop="serverRoot">tests/plugins/hosts/selector-handler</td>
            </tr>
            <tr>
                <td>Address</td>
                <td itemprop="bindAddress">127.0.0.1</td>
            </tr>
            <tr>
                <td>Port</td>
                <td itemprop="bindPort">3000</td>
            </tr>
        </tbody>
    </table>
    
    <table id="host-localhost" itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
        <tbody>
            <tr>
                <td>Host Name</td>
                <td itemprop="hostName">localhost</td>
            </tr>
            <tr>
                <td>Host Root</td>
                <td itemprop="hostRoot">tests/plugins/hosts/selector-handler</td>
            </tr>
            <tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                <td>Selector Handler</td>
                <td itemprop="library">file://./plugins/librusty_beam_selector_handler.so</td>
            </tr>
            <tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                <td>File Handler</td>
                <td itemprop="library">file://./plugins/librusty_beam_file_handler.so</td>
            </tr>
        </tbody>
    </table>
</body>
</html>"""

with open("tests/plugins/configs/selector-handler-config.html", "w") as f:
    f.write(selector_config)

print("Fixed selector-handler config")
