 tccli configure set secretId $1
 tccli configure set secretKey $2
tccli dnspod CreateRecord --cli-unfold-argument --Domain $3  --SubDomain _acme-challenge --RecordType TXT --Value $CERTBOT_VALIDATION --RecordLine "默认"

sleep $4

