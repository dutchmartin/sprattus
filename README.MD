# async Rust ORM for Postgres
###### The Future .awaits

Tweede Golf wants to have a good async ORM to use in their clients web applications.
Since there currently is no rust software library that complies with the features described below, we need to implement it in some way.

### Features wanted
* async with as few locks possible
* Easy mapping from sql query to struct instance, preferably as a zero cost abstraction.
* Utility methods for commonly used database query's

### Options for having a async struct mapped database connection.
* Modifying [Diesel](diesel.rs) to have async support.
* Adding the required features to [tokio-postgress](https://github.com/sfackler/rust-postgres)


### Motivations for deciding on what to implement
In order to decide what we want to do, we need to set requirements and lay them next to the proposed solutions.
Then we need to make a decision is that solution is worth the time being spend in there. 

#### modifying diesel

**task list for doing it**
* Adding tokio-postgress to diesel. ([Sean Griffin thinks different about this.](https://github.com/diesel-rs/diesel/issues/2084#issuecomment-512983289))
* Search for a way to give everyone access to a connection pool that can be used in async way. This might involve writing a custom data structure.
* Add futures as a return type for all database operations.

##### pro's
* You get all features Diesel currently provides.

##### con's
* Implementing it correctly is not possible according to the authors [[1]](https://github.com/diesel-rs/diesel/issues/2084),[[2]](https://github.com/diesel-rs/diesel/issues/399#issuecomment-518422793)

    There are solutions, but those are either hacks or suboptimal for providing a usable public api. The best way to implement it is to patch the compiler so a more advanced lifetime can be used.
* The api to implement will have to use as much of the current api to not be a maintenance burden for the current maintainers.
* Breaking changes [need a good explanation](https://discourse.diesel.rs/t/diesel-2-0-what-goes-here/11).

#### Adding features to tokio-postgres

**task list for doing it**
* making the package fully async await compliant.
* figuring out a api that works for everyone.

##### pro's
* Freedom in implementing a solution tailored to Tweede Golf's needs.


##### con's
* Potentially a lot of work since you need build things that are already in Diesel.
* Implementing this could face the same difficulties that Diesel has in implementing it.
* Async await support is experimental and requires changes to the crate to work properly. The current branch for that is not ready yet.

###### notes from implementing examples
tokio-postgres:
* A Client has to be mutable, so it has to be thread safe.
* The syntax for async await is only half implemented.
* There are no implementations for complex types yet like [bigdecimal](https://github.com/sfackler/rust-postgres/issues/307)

diesel:
* setup takes time and is hard to get for first timers.
* Documentation is not up to date.
* Is still not fully updated to rust 2018.

#### API Concept. 
```rust
async fn controller_method(DbClient : Client) -> impl Response{
    DbClient.get()?.prepare("Select x from y")
        .and_then(move |statement|{
            dbclient.get()?.query(statement, /*args*/ &[])
        })
        .and_then(move |rows|{
            rows.iter().map(move |row|{
                X::from_sql(row)
            }).collect()  
        })
        .and_then(|data|{
            Template.render("page.html", data)
        })  
}
```