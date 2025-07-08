#!/bin/bash

echo "Testing authorization for root path / vs /index.html"
echo

echo "=== Test 1: PUT to /index.html with selector ==="
curl -X PUT -u admin:admin123 -H "Range: selector=h1" -d "Modified Title" http://localhost:3000/index.html -w "\nStatus: %{http_code}\n" -o /tmp/test1.html 2>&1
echo "Response saved to /tmp/test1.html"
echo

echo "=== Test 2: PUT to / with selector ==="  
curl -X PUT -u admin:admin123 -H "Range: selector=h1" -d "Modified Title" http://localhost:3000/ -w "\nStatus: %{http_code}\n" -o /tmp/test2.html 2>&1
echo "Response saved to /tmp/test2.html"
echo

echo "=== Test 3: GET / ==="
curl -X GET -u admin:admin123 http://localhost:3000/ -w "\nStatus: %{http_code}\n" -o /tmp/test3.html 2>&1
echo "Response saved to /tmp/test3.html"
echo

echo "=== Test 4: PUT to /index.html without selector ==="
curl -X PUT -u admin:admin123 -d "Full file content" http://localhost:3000/index.html -w "\nStatus: %{http_code}\n" -o /tmp/test4.html 2>&1  
echo "Response saved to /tmp/test4.html"
echo

echo "=== Test 5: PUT to / without selector ==="
curl -X PUT -u admin:admin123 -d "Full file content" http://localhost:3000/ -w "\nStatus: %{http_code}\n" -o /tmp/test5.html 2>&1
echo "Response saved to /tmp/test5.html"