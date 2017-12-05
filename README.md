# http_port

PostgreSQL async HTTP request and response

This will start http_port to listen PostgreSQL notifications. Its payload describes HTTP request.
Callback will be executed with response as $1.

```
  $ http_port http_port.conf
  
  NOTIFY test_notifications, '{"method":{"POST":{"body":{"hello":"world"}}}, "url":"http://httpbin.org/post", "callback":"insert into syslog(msg) values ($1)"}';
```
