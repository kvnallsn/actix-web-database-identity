Version 0.3.1 (XX July 2018
======
Added `Clone` trait on `Sql::Variant` and `SqlIdentityBuilder`

Version 0.3.0 (05 July 2018)
======
* Added new database fields (created, modified, ip and user-agent)
* Improved logging support (migrated away from println)
* Improved error handling, more error types added
* Added new `finish` method on `SqlIdentityBuilder`
* Added ability for `SqlIdentityBuilder` to auto-determine SQL variant
* Changed variant-specific methods to override auto-determined values

Version 0.2.1 (01 July 2018)
======
* Changed default token response header to `X-Actix-Auth`

Version 0.2.0 (30 June 2018)
======
* Deprecated old identity-policy building style in favor of new `SqlIdentityBuilder`

Version 0.1.2 (30 June 2018)
======
* First public release
