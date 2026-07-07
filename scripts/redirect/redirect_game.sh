# Disable SELinux enforcement while the redirect rules are active.
setenforce 0

# Original destination IPs for blhxjploginapi.azurlane.jp and blhxusgate.yo-star.com.
DEST_IPS="139.95.1.28 35.78.1.22 54.178.121.232"
DEST_PORT=80

# Redirect game traffic to the local Cheshire dispatch server.
REDIRECT_IP=127.0.0.1
REDIRECT_PORT=21180

# Rewrite outbound TCP/UDP traffic for the selected destination IPs.
for DEST_IP in $DEST_IPS; do
  iptables -t nat -A OUTPUT -d $DEST_IP -p tcp --dport $DEST_PORT -j DNAT --to-destination $REDIRECT_IP:$REDIRECT_PORT
  iptables -t nat -A OUTPUT -d $DEST_IP -p udp --dport $DEST_PORT -j DNAT --to-destination $REDIRECT_IP:$REDIRECT_PORT
done

# Masquerade rewritten packets.
iptables -t nat -A POSTROUTING -j MASQUERADE
