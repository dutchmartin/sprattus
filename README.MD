# async rust orm for postgres
###### The Future .awaits



### Options for having a async struct mapped database connection.
* Modifying [Diesel](diesel.rs) to have async support.
* Adding the required features to [tokio-postgress](https://github.com/sfackler/rust-postgres)


### Motivations for deciding on what to implement

#### modifying diesel

##### pro's
* You get all features Diesel currently provides.

##### con's
* Implementing it correctly is not possible according to the authors [[1]](https://github.com/diesel-rs/diesel/issues/2084),[[2]](https://github.com/diesel-rs/diesel/issues/399#issuecomment-518422793)

    There are solutions, but those are either hacks or suboptimal for providing a usable public api. The best way to implement it is to patch the compiler so a more advanced livetime can be used.

#### Adding features to tokio-postgres

##### pro's
* Freedom in implementing a solution tailored to Tweede Golf's needs.

##### con's
* Potentially a lot of work since you need build things that are already in Diesel.
