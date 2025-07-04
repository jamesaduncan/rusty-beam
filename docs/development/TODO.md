# Rusty-Beam TODO List

- [x] Fix PUT bug that exists in more complex examples - specifically, when the HTML that gets PUT in the HTTP body is a <td>...</td>.
- [x] What is that extra byte? (Fixed: dom_query library was adding trailing newline)
- [x] Pluggable authentication - Basic Auth implemented
- [x] Google OAuth2 authentication plugin
- [x] Authorization with paths, methods and selectors
- [x] Update the configuration so that the Host header can direct the server to the relevant director
- [x] Gracefully handle failures to start when there is already something listening on the socket
- [x] Fix compilation warning about unused host_name field in HostConfig struct
- [x] Make sure the server is compliant with the HTTP spec (except intentional Range header design).
- [x] Refactor the code base to split it out into multiple files. One single file is getting unweildy.
- [x] kill -HUP the process should have rusty-beam re-read all of the config files
- [x] Write a library to extract data strutures from webpages that have itemprop itemtype itemid etc, including nested structures.
- [x] The config file should be a cmdline parameter & have no default. rusty-beam should fail if unspecified.
- [ ] The "realm" for basic authentication should be configurable in the config file.
- [ ] The content of the Server header should default to rusty-beam/version, but it should be configurable too
- [ ] The plugin interface should be unified [See * below for more info].
- [ ] The various plugins make a lot of noise on STDOUT
- [ ] The apache access log plugin isn't recording the user who accesses the page correctly.


* The type of plugin should only really relate to WHERE it is called, not the method it is called with. Basically, all plugins should take a Request, and handle it; I think so far in the case of all plugins, an Ok that would return basically nothing and processing would continue, or an Err that would return a Response object, with the various appropriate error codes and so on. Really, all of the response handling should be
done like this. What is required is a need to name the phases of the request in a meaningful way, so the plugins can bind to the phases, rather than particular "types" of plugin.
