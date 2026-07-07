# Original destination IPs for blhxjploginapi.azurlane.jp and blhxusgate.yo-star.com.
DEST_IPS="139.95.1.28 35.78.1.22 54.178.121.232"
DEST_PORT=80

# Remove game traffic redirects to the local Cheshire dispatch server.
REDIRECT_IP=127.0.0.1
REDIRECT_PORT=21180

# Remove outbound TCP/UDP rewrite rules.
for DEST_IP in $DEST_IPS; do
  iptables -t nat -D OUTPUT -d $DEST_IP -p tcp --dport $DEST_PORT \
    -j DNAT --to-destination $REDIRECT_IP:$REDIRECT_PORT

  iptables -t nat -D OUTPUT -d $DEST_IP -p udp --dport $DEST_PORT \
    -j DNAT --to-destination $REDIRECT_IP:$REDIRECT_PORT
done

# Remove masquerade rule added by redirect_game.sh.
iptables -t nat -D POSTROUTING -j MASQUERADE

# Re-enable SELinux enforcement after cleanup.
setenforce 1
