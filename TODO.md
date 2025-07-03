# Rusty-Beam TODO List

- [x] Fix PUT bug that exists in more complex examples - specifically, when the HTML that gets PUT in the HTTP body is a <td>...</td>.
- [x] What is that extra byte? (Fixed: dom_query library was adding trailing newline)
- [ ] Pluggable authentication
- [ ] Authorization with paths, methods and selectors
- [x] Update the configuration so that the Host header can direct the server to the relevant director
- [x] Gracefully handle failures to start when there is already something listening on the socket
