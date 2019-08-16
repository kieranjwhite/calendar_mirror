/*
Copyright [2019] [Kieran White]

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

/// Create a state machine
///
/// The stm! macro allows the user to declaratively define a state
/// machine.
///
/// # Usage
///
/// ## State Machine Declaration
///
/// The example below demonstates how the macro should be invoked. 
///
/// The first argument (example_stm) in the example is the name of a
/// sub-module that the macro will create and populate with various types
/// in order to minimise pollution of the parent module's namespace.
///
/// The second argument (ExampleMach in the example) is the name of the
/// enum that will represent the state machine. There will be a variant of
/// this enum corresponding to each state. Each variant is a tuple struct
/// where the first argument is the state. There can be an arbitrary
/// number of subsequent parameters for the variant and these correspond
/// to state specific arguments. The user initialises the state machine by
/// invoking this enum's associated constructor method, new.
///
/// The third argument (StatesAtEnd) is the name of another enum, again
/// with tuple struct variants corresponding to each state, however this
/// enum's variants only encapsulate its corresponding state; there are no
/// additional parameters.
///
/// The fourth argument (AcceptingStates) is yet another enum but this one
/// only includes tuple struct variants corresponding to accepting
/// states. The tuple struct variant have only one parameter, the state.
///
/// Next a series of state transitions are defined. The first of these
/// lists mappings to the starting state. Following that are the mappings
/// to every other state.  The user can indicate which states are
/// accepting by appending an |end| tag to the state mapping. 
///
/// ## Initialising the State Machine
///
/// When initialising a state machine the user provides two arguments.
///
/// The first argument is a tuple containing the state arguments specific
/// to the starting state.
///
/// For the second argument the user must provide a closure that is run
/// when the final state is dropped. This closure is passed an enum
/// instance of type specified by the third argument in the macro
/// invocation. This instance indicates the state machine's final
/// state. The closure must return an instance of the same type specified
/// by the macro's fourth argument. If the closure's argument is not an
/// accepting state, typically the appropriate response is to panic!
///
/// # Example
///
/// The example below demonstates how the stm! macro is used. The only
/// state a user can instantiate is the starting state. Other states are
/// created by performing a state transition using Rust's From
/// trait. Invalid state transitions are caught at compile time.
///
///
/// ```
/// use reqwest;
/// use std::path::PathBuf;
/// use ExampleMach::*;
///
/// pub struct RefreshToken(String);
///
/// pub enum Event {
///    Refresh,
///    Quit,
/// }
///
/// pub struct Appointment {
///    ...
/// }
///
/// stm!(machine
///    example_stm, ExampleMach, StatesAtEnd, AcceptingStates,
///    [ PollEvents ] => LoadingConfig(PathBuf), {
///    [ LoadingConfig, PollEvents ] => Retrieving(RefreshToken);
///    [ Retrieving ] => PollEvents(RefreshToken);
///    [ PollEvents ] => Quitting() |end|;
///    [ LoadingConfig, Retrieving ] => Error(String) |end|;
///    }
/// );
///
/// fn run() {
///    let mut mach = ExampleMach::new((PathBuf::from("/var/opt/calendar_mirror/refresh.json"),),
///       Box::new(|end| {
///           match end {
///               StatesAtEnd::Quitting(st) => AcceptingStates::Quitting(st),
///               StatesAtEnd::Error(st) => AcceptingStates::Error(st),
///               _ => panic!("Not in an accepting state: {:?}", end),
///           }
///       })
///    );
///
///    loop {
///       mach = match mach {
///          LoadingConfig(st, path) =>
///             match load_config(&path) {
///                Ok(refresh_token) => Retrieving(st.into(), refresh_token),
///                Err(error) => Error(st.into(), "Failed to load config"),
///             },
///          Retrieving(st, refresh_token) => {
///             match download_appointments(&refresh_token) {
///                Ok(apps) => PollEvents(st.into(), apps),
///                Err(error) => Error(st.into(), "Failed to download appointments"),
///             },
///          },
///          PollEvents(st, refresh_token) =>
///             match read_event() {
///                Refresh => Retrieving(st.into(), refresh_token),
///                Quit => Quitting(st.into()),
///             },
///          Quitting(st) => break,
///          Error(st, message) => {
///             eprintln!(message);
///             break;
///          }
///       }
///    }
/// }
///
/// fn load_config(config: &Path) -> Result<RefreshToken, IO::Error> {
///    ...
/// }
///
/// fn download_appointments(refresh_token: &RefreshToken) ->
///    Result<Vec<Appointment>, reqwest::Error> {
///    ...
/// }
///
/// fn read_event() -> Event {
///    ...
/// }
/// ```
///
/// # Rendering the State Machine with Graphviz
///
/// The stm! macro can optionally generate methods for creating a graphviz
/// dot file. Firstly the project must be compiled with the render_stm
/// feature enabled:
///
/// cargo build --features render_stm
///
/// At run time the user can check that this feature was enabled and subsequently
/// proceed to generate the dot file as follows:
///
/// ```
/// if cfg!(feature = "render_stm") {
///     let mut f = File::create("example_machine.dot")?;
///     ExampleMach::render_to(&mut f);
///     f.flush()?;
/// } else {
/// 	...
/// }
/// ```
///
/// As we are using the ? operator, the above code will only work in a
/// function that can propagate an std::io::Result error.
///
/// Next run the application to generate the dot file. Now an svg file of
/// the graph can be generated with the shell commands:
///
/// cat example_machine.dot | dot -Tsvg > example_machine.svg
///
/// The dot command is provided by the graphviz package. The result is an
/// image similar to the following:
///
/// <div>
/// <img src="data:image/png;base64,
/// iVBORw0KGgoAAAANSUhEUgAABAIAAALvCAMAAADI9JfhAAAC2VBMVEUAAQAAAgABBAACBQEEBwIF
/// CAQGCgUICwcJDAgLDQoMDwsNEAwOEQ0QEg8RExASFBETFBITFRMUFhQWGBUXGBYYGRcZGxgaHBkb
/// HBobHRsdHxweIB0fIB4fIR8hIyAiJCEjJCIjJSMlJyQmKCUnKCYnKScpKygqLCkrLSosLSssLiwt
/// Ly0vMS4wMi8xMzAyMzEyNDI0NjM1NzQ2ODU3OTY4OTc4Ojg5Ozk7PTo8Pjs9Pzw+QD0/QT4/QT9A
/// QkBBQ0BDRUJERkNFR0RGSEVHSUZISUdISkhJS0lKTElMTktNT0xOUE1PUU5QUk9RU1BSVFFTVVJT
/// VVNUVlRVV1VWWFVXWVZZW1haXFlbXVpcXltdX1xeYF1fYV5gYl9hY2BiZGFiZWJjZmNkZmRlZ2Vm
/// aGZnaWZoamdpa2hqbGlrbWpsbmttb2xucG1vcW5xc3BydHFzdXJ0dnN1d3R2eHV3eXZ4end5e3h6
/// fHl7fXp8fnt9f3x+gH1/gX6Agn+Bg4CChIGDhYKEhoOFh4SGiIWHiYaIioeJi4iKjImLjYqMjouN
/// j4yOkI2PkY6Qko+Rk5CSlJGTlZKUlpOVl5SWmJWXmZaYmpeZnJianZmbnpqcn5ydoJ2eoZ6gop+h
/// o6CipKGjpaKkpqOlp6SmqKWnqaaoqqepq6iqrKmrraqsrqutr6yusa2vsq6ws6+xtLCytbK0trO1
/// t7S2uLW3uba4ure5u7i6vLm7vbq8vru9v7y+wb2/wr7Aw7/BxMHDxcLExsPFx8TGyMXHycbIysfJ
/// y8jKzMnLzsrMz8vN0MzO0c7Q0s/R09DS1NHT1dLU1tPV19TW2dXX2tbY29fZ3Nnb3drc3tvd39ze
/// 4N3f4d7g49/h5ODi5eHj5uLl5+Tm6OXn6ebo6ufp6+jq7enr7urs7+vt8O3v8e7w8u/x8/Dy9PHz
/// 9vL09/P1+PT2+fb4+vf5+/j6/Pn7/fr8//v9//y7AWlWAAAAAWJLR0QAiAUdSAAAAAlwSFlzAAAu
/// IwAALiMBeKU/dgAAAAd0SU1FB+MIChIcHgATfwkAACAASURBVHja7J33TxT5/8e/r2QyWfqGohvY
/// INGNQizEQlAxguWU01OI9VDvVMSGXTzLBztZC3BWiP6Ap6euSsRgwRINp2ziET0sGxEFBYQNLCUs
/// +/4PvjPbWGArbJ/X84fZ2ffMe+Y9r3m/H/Pu7/8jKBSKw/o/NAEKhQhAoVCIABQKhQhADVLfSivR
/// CChEAGd1lgKYo0Q7oBAB3FQ1MKLy0BAoRAA3dZ1FAKSgIVCIAG7qqToXsAkNgUIEcFNdoymgqCo0
/// BAoRwFHJd/slYZMAChHAYYEEbYBCBCACUChEACIAhUIEIAJQKEQAIgCFQgQgAlAoRAAiAIVCBCAC
/// UChEACIAhUIEIAJQKEQAIgCFQgQgAlAoRAAiAIVCBCACUChEACIAhUIEIAJQKEQAIgCFQgQgAlAo
/// RAAiAIVCBCACUChEACIAhUIEIAJQKEQAIgCFQgQgAlAoRAAiAIVCBCACUChEACIAhUIEIAJQKEQA
/// IgCFQgR4NALu7N88s37g3uW/hw69h+8EhQhwNwT8OpaQH7EbCCkSAYzKNX3ij+UQRciqaaaON/1v
/// 3MzfFmaZON45MugdwDTjt0ahEAGuQoAUIgk5BauY3Y0Ar82dmgqbCRlKtRg/WhooekfIO94K44f3
/// whayFHp5Nrg1CoUIcAoCrovlfVzugpiQHVDO7I5nk6RpqQKhlJCX940fvU8JvrG/vxUaPdzNh4q+
/// ng1ujUIhApyCgDPA2/Spl8tluolJt+PYjDxAhrmrVQDVZvJgc5D2ZkfeGT1eBn6qvm49t0ahEAHO
/// QYCqeArAAsPv7sm1zGZ+EZtDALijdWzYsHlGagOz8/b3vL0QyiTeokVH+JBIds8Z8Rf5h+9Hl1cm
/// +G1hrncpbs1YoUjUcADG9FxT693gvAQ+BE2iWc89XgxujUIhApxWFyBdQsGEm926v29qmc3jTmaz
/// CkBbSngVtor55iczGXf/IhILG4hyHV1JpsMJ0ukDNaRhFYTJUjMhhJANsIM8Bn9ChrOZetLbu8F5
/// JA7+p/Hc48Xg1igUIsDRCGj88OGDTLP7ZbsvTOp/9hCI0+y0CuAzKQZfoggNVxEePCWHIYt00VBJ
/// voCQsOX3tIzuHPiVtABTwt8LE0grwL+66+i995xHOinmKmrPei8oFCLAmQhIBYAYzW7LSQGM6nfy
/// W4Bszd5umE7IUYglexiX18Dvfk9R9aQE+ISch9XMCaNA8LkrDIqJMhjePPWn7pIafQ7CwHvPeeQF
/// UB0az3ovKBQiwJkIEDIIWMnu1GzzgfBTrf1OPgnauvkb4yGHkEmwjfHzkmxiPuKHIYGQabCYkLnw
/// NyG1AKdJEQhVhHwKgsC5L5mETYG+uU/v3eC8HJii9az3gkIhAlxQF1CxCGD8daWRkxPBV+189JgP
/// sJ/14OYfAC0NYXDp+WzYSp7QUPiok4bvUnIBaDmZBSdaWsn81VrfP7HfekIaFiuJzrvheclwgGg8
/// 93hBoRABTkdAIcD8F0bPraVgBvuNXg3/zYJXJCXoKZED/HGBD+svpUFUTl4A/HbvJdCFMjIHlpF2
/// Csr3djQCzL2hbuyrCRr6oONrUQyTxnXeDc4jgfCEaDz3eEGhEAHOR8BGmfFTX40C4C1fOBZASL6n
/// TEtfU8c4HvSZpcgQ/kk+jfbdQlL5haQ+UviM/ZqXka443rI2okpiihcjHrMXaMwYSYu2sA2JOu8G
/// 51UB3aHxbOgFhUIEOL0gYFfdDZvdWBQFAosn5sIKW72gEAEot0cAH5jPerm6ndCMDuWRmWzvYBu8
/// oBABKE9AQCykVVdvgEvmz/KLfEOtt80LChGA8gQEfE0bQgnmPbFUCvAP291lmxcUIgDlCQhAoRAB
/// iAAUChGACEChEAGIABQKEYAIQKEQAYgAFAoRgAhAoRABiAAUChGACEChEAGIABQKEYAIQKEQAYgA
/// FAoR4KkIaEWjohABHEBA+SyAGdOmbGro7XxyKqXZyQwCKnlW7Ky+1/8za9qUj2h3FCLA43MB39hF
/// hBsS/aU9TrWEdAaB9k89DCekayuc6uUrn9fdslCKdkchAjy/IAAiZlMFC/UONVOYjQh6HVfSvdch
/// FY1Am6MQAV6EAAVM1P2vGyXqjwASzO/liSdCm6MQAV6EgCewnchSstKmvCVHwT+DRUBjSuAoqe64
/// BHar5yUnitPMpjSDOSejDc2OQgR4AwKGq5qLI3w/kKhhROkfrU3zIsiuKVUvPQh+q36NCyhkTx0G
/// ug1gLgCFCPAWBADQ4WtkhOReZ9I3pUcAswmi2eNRX96Xrad3qLSOIkQAChHgdQUBtdrOHRGAIQIM
/// UvsZdklxRAAKEeDFCJAKH6jTt1EENLCFAkQAChHgxQgQCQgZziIgyggC3rCthiLoIiQMEYBCBHgR
/// Ar5AhHbPD8quBUNFbRTvKyFDQMGm9jbSCeFsJiCOkhKyAA7IcgPhoUoOkWhzFCLAGxDALkG+UbMG
/// 2Dm/CRX5AfOb/wi7TU4CbG07AbDjegrAxNlxoqVVzBmyibyZsilpN15nABx6g1ZHIQK8oSCAQiEC
/// EAEoFCIAEYBCIQIQASgUIgARgEIhAhABKBQiABGAQiECEAEoFCIAEYBCIQIQASgUIgARgEIhAhAB
/// KBQiABGAQiECEAEoFCIAEYBCIQIQASgUIgARgEIhAhABKBQiABGAQiECEAEoFCLAsTqZxQjms9t6
/// tAYKEcA5rQdKI+B3ozVQiADO6QVoRW1DY6AQARzUEB0DpGgLFCKAg9pNaQggQFOgEAFcVKW2HHAA
/// TYFCBHBSURoGVKElUIgATuqwuiQwEg2BQgRwU5/U5QAxGgKFCOCoxrIMqEE7oBABHFUuUxKYhGZA
/// IQK4qnqmHHAezYBCBHBWCQCNaAUUIoCzugSz0AgoRADX1Pnl5d2LB7My01fNhwnL0jOy9uXfeP6+
/// BQ2DQgR4u6pLT62dymfbAQJEsVOS5qQKfpmfND12dBjrRI9dcuhmZSdaCYUI8Ea1PTuaHAQQNm19
/// fknFV11Cb9D+qurfPry8e14UBVTc9tvf0VwoRIBX6fWRSRQIFuW+tJTb76oqyogBiNx4H3MDKESA
/// d0hZumYIDFlzs9ZqH4qy/WOB/rmgGY2HQgR4ut5sC4GJR17b7K+ucD5NpdxTogVRiADPVceFsRB5
/// 8PMAfSsuT4WQXV/RjChEgGeq6XAw/dvzQV3i88GhVNobNCUKEeB5atjMC9zXMOjLdBVFw+wKNCcK
/// EeBhRYBjPoL8Nvtc6/4UWPwZTYpCBHiQ/hL4HGm33+WKR1A7sfMgChHgKfqeDGn2XR1EWRAcehcN
/// i0IEeIQkASPtPy24PB1S5WhbFCLA7dU8B3Y5pGefhB+B1YIoRIC7qypK8NJBl26YQxehgVGIALfW
/// ff84B64Rmg3p2F0QhQhwYxXC6i5HXv8qvQAZgEIEuDEBDjn4DuU+yAAUIsBddRkOOvwe//gsRAag
/// EAFuqWLIHphHm3r9vOD9jqZGIQLcUB990405P0sFWPePaW+dx+KA+Zm4y9r7lMIFNDYKEeB2Uoya
/// YLw7QAdEmPXYGcgiYIn16wtnU+VobhQiwN20im9qZD+IzPsUgW13Us0VtKK9UYgA99ILuE2chADS
/// FLQdDY5CBLiVukfPIJYR0Lp7z/aZ2+WEyFKy0qa8ZQoJ29P3/yEAopKsnErupgvkK4Oi2fEFZ35d
/// zy48buqKF6m3aHIUIsCddIaSWUaAYvhBQhqHR7aQqGFE6R9NuieuJaSaYpL6V+akOh4c+3IVJjJX
/// g2aSAztMFwViE9HkKESAO2UCIrYSywjYB2zn4Suwm+ReJ2QYRc7Ce8ZhOGhPGsHuhFCEzAMlqTK3
/// 5vAzkKLRUYgA91ExfLQCAQnAziNUA5MJaTt3hMn/z4NOXV0Ae5J6h92cgWLyCfaaueG4VWh0FCLA
/// fZQ0h1hGQONUqGJ+OmEukQofsGk9FuqMIoBcCtw5/3C3mWsWUA1odRQiwF30De5agYCUbBAzPzLI
/// JSKBOv+/HAqMIqB7q8zCHdt9z6HZUYgAd9E1ysxMod9hKPvTmv5rR7SgnpAt8UriB2XXgqGilAp6
/// 2PHUF2qIAoYQImQRMBSU5PCwyw9fyszlAkhyKpodhQhwF6XHmz72dD6AaNq0ERQUEcXumTt2H+4i
/// 5JzfhIr8gPnNL+J9IsVTM54o/gDIFQMcbc0D2NNZFsKuNMy/beaWJ/lodhQiwF00Yq99r3f5BCGq
/// 2ivBZk6RqusVUChEgBuoC27Z9XpiUC8o+nmsmXOUcBMNj0IEuIc+2bmVPhmONRHyepHZ73z4CTQ8
/// ChHgHpJCtV2v17w5ko5LLTQ/NcjovWh4FCLAPVQO35x/07itaHgUIsA9VAE1zr/puD363bZnx+eV
/// 4WtAIQJcpffggvW/o46pfz4WbRgFQOGYARQiwHVSQKnzb0pfUZTnpQSwyZ8V9hdGIQJcpyFiy+dI
/// ksfOnLdRvNP40T+zpk35aP38gUR5CoS61M+KwpeAQgS4TovnWjrjx7QodkHAq0GrjR7O53W3LJTa
/// MH8gOU71pH9GQnwJKESA63Tex9Lc/vGB6t4+5NkSo4dFI2y95a8zv2w2oIB/WFK6WCJV4LtAIQJc
/// oHdgYSnRW6DryGO83z9PZOMdVUOPEtKQpYMANev471OHMjuChDU5kkokAQoR4FR9GbHa/AnL4HXP
/// H90Mgvq5AkszwD8jQ8HOH2hx2kCt7sEH9qcxm1ZDgDrC/mt/e+fEusQI5n9I/Mojf0vl+GZQiABH
/// q7U8LzUUptJNZs8aDz3TfutnEOyZK1AzpQA7f6DFaQO1mj1LV8mQzWMhYLjquLK6rCArNZYGCIhN
/// E5dUq/AtoRABjpCyqiBtJEBYcnZJrd8ps6dOgp4lx3tmENTPFaidVYTdWpw2UC0ZlOj3m/YzEHhm
/// JHiyB7npU/kAPuNXiotlCAIUIsCO+lS0IZaCgFkHSjQN8juCzea6f4en+v2eGQR7JgrrQYDlaQNZ
/// LRpuOJ+I/FDAZ5OnyqVFWakMqaiRbI4A3xwKETBYdb/OTw0DalLmVYMpQ1vDNpvzUwTH9fvT9TMI
/// GkOA5WkDCbtuSZ++SK2WWiTapJe2JgYDBCduvSTtwJeIQgQMTO3l4uQA8EnKLuvol8grzfhTxYZ8
/// V+90FpGDPTMIGkGA5WkDmSx+zPyBBV9eXpAZzwOIZMoujRidUYgAm9RQkhVPQVhqntRosTpurLmP
/// 6/uIyDvdpONpYgXpmUFQP1egHCIJ0cwfaMW0geQP+vMgnkMlu7lndgiAcMHhe7UYp1GIAGvUfHuj
/// CCBm49+m00wNf7m5KyjEc4XRY/axHYR0Mwie080VKM0AOPSmnZ0/UGHFtIHFcGnwT/St9GhKJFMw
/// mJNdivkBFCLAbDH64e5YgNhdpS3mzyuD83a4m+VpAz/6rbfXo7U8y10WxeQHUk8+w95EKESAsVK3
/// VJxEQWS6pMmKkw9Rgx8waHnawG9Rk7rsm8N5eGReKMCo385XdmMURyECelRdkOoHoakFX6z1sHrw
/// DLA4beD3ESMdMS746+2sab7gk/S/h60YzVGIAEIa/1oaBPxFF2U2+cqgSgb7SbYwbWDDyOGOm6Ss
/// uih9JEBkWgHOT47iNgKkhycCNf1kpc0eVb9RRQ4NWdWwEd8d++z1d3bEURDyS64USwUoTiKgrSR9
/// KASnSVoG5Fv1B2xzYNK56zPFGbMDdZaf/IUPfsmnEAMojiGgOi+JgtjswczD9zc9s8lBoVMdgbVd
/// zrNFQZoAeEnZZV0Y71GcQEB7cboAglfcGOwIW6kgtMQxaXIqddbJNqk6mxoMPnPEr3GMEcrLEdBe
/// kuYDsVnl9ojqLemQ2mz/IBbwol+7wjRV5xeHAH/J5a8Y/VHeioDGwtkUNavAfqXsu6FDr9o5jJUJ
/// 1F7X5cirC1J92N4ROAcJIsD7HulrQTJFJxfYt5Ns02qIfWbH69WugokuXiigs2z3WKCmHH6FNYSI
/// AC/S57x4oJOLHNA59u0smP+vvXIpe2nhDXcw1w9JegQEphbgmgWIAK9QVXYsBKaVdDro8g/HQtIj
/// O1xHlkHzT3W6jdXen5pOUVPF2HkIEeDh+m/fMBBsfubQXG1ZEoweZA5D9SQFhp1rdy/btZWkDwFh
/// ugRHFiECPDb/fywGBDsqHH+jf5dRvBVPBtzM8HF/OEyUuGPhW/Vq31jgzS/EIgEiwPPUVBDP5v+d
/// lLCazoyH8J3PlLb7fH8yDobuduMMd23BPBoS8rGxEBHgSeqUJFN0aonSmfes2iMC/8V/2fLBbC/b
/// MgyCVjx099r3jpI0PxiZ/Q5TBiLAI6R6stKHmidxQdXa54JkCsKSxeVW3Puben6yyMwypUcYtbs8
/// MxRGZpVj4kAEuLveZ0fCSLHLCq+KRwdn+wE1elH2jUrjQ/I7P5WeXBMXCNSEzOseNadf99NNQyFq
/// 92tMH4gA91Vr4UQQ7HF1wVpVVZT1y3AKgBbGzV+XlSUWny8oOCnen5W5dKrIHwCGJm44+8Ijp/h+
/// tSsSRIdkmEQQAW6p8lU8elmZuwx1Ub5/UCTeujwpNjYyMojnFxEZEzv1l42HCu5KPXvCnqqsMCab
/// 9R1TCSLAzVSfF8PEzB9uGjqQeFH0UJWn+0J83g9MKIgA94mUD+ZT/hvduJTqVQggbJPLAopeeLsT
/// 0woiwB3UKI6EhKtuXbb2NgQwailKpvzSpZhaEAGuljSd9k1/6+aB9EIEMPomHgFjz+DQYkSAC9V6
/// LhpiL7e7fTi9EwFqAPOo1BIcWYwIcI0+ZQXQqR7RW8VrEUBIhyQJBFmfMdUgApyusrkgPOUhuVAv
/// RgCjdztDYKZEiQkHEeBEdRZFQ7znxDrvRgAhyjvJMORwPSYdRICTVLsrgF79xoMs6u0IYPQtm0+l
/// lmHiQQQ4QW9XUEOOeVbHFA4ggO0sEAfjCtox/SACHKvyZIjK87Q+KZxAAFG30PplYtUgIsBxUpVM
/// gvgSz7MoVxBASMORoTD/CaYhRIBjcpoXhkFKhSdalDsIIEQpmQqxElyYCBFgd7XnDqUyZJ5pUS4h
/// gFFlGgjzOjAhIQLsKUVeGC+zzlMtyjEEEFKdSQdnN2NSQgTYS63iQF6mB49R5xwCCGnIZl4ZTjqK
/// CLCLfmT5BB7y6NEoHEQAm3ETUGm4GAkiYNCSH/Dhiz18PQtOIoCQrssiSEUIIAIGpTZxgE9Wq6db
/// lKMIIER1OwaWyzBJIQIG/BkpCOVlecGAdM4igFHJaEhFCCACBgaAM2G83U3eYFEuI4CoJMOptGpM
/// VogAmyWJotK9ZKZaTiOAhcAwKv0bJixEgE16GQ+pXtPbnOMIYDJ05wX0jkZMWogAq/UuFZL+9R6L
/// ch4BhHTmhfocwoXKEQHWqS4dRt3zJosiAhi1HfULOYtzCyECLKv9AB1xxbuGmSAC1GrOoiMKcAAR
/// IsCCrgt8T3jbGhWIAK2+pkP0PTQDIsCMKqdCmvfNQYcI0KsqFZJwARJEgMmcYibEvvRCiyICDPRy
/// CvYVQgQYl7KAH+adRUVbEVD36M+BV55LJe4+sWJZDJWO0w0jAvqpPIbKbPVOi+oRMP8wyU0g5NF0
/// gHXvzaTiuFAmoQhNJZN7KRPmjAQoNcrRFcdeQW+f2pual/akilngl7F6nak+mUbDZCqgpu+rKhrC
/// y2rBlIYI6FUGWAOzvDZ7qENAF7wgCXuZnQewzKyH+DSGE0LjU6Q0JgofMz95YLS/nXgY6d7Uy6f+
/// puakP+kKMNucX3odbNPvGQ2TiYCavW+72D9IjCsTIwJ6JAkOLfJei+oQ8FlESBTbXT7H+Cdcn0Co
/// KyaPNQ6L0HSzm2zUp19WXyf9TfW6WdRl8qSNUM7kQnwMjxVdG9BD979vb+Zn0eEFuBohIkAjWRKk
/// efM0UzoEvMgl5Gd2ZxrVZu78h2B6cEQiaJdRNFpvWgT9qtv1N9WrNGjoKYWJk6L8lQyQRxgWO6iG
/// AT10//v2UW06jMJ6UkQAo4591DjvbifSIaCyiXSwCVhBTdcden/sUoo6R1Cad0m8Yj2zU3gtNzWa
/// tOTAESLbFn+yatxotkudMj/r1BhIICXwk8GFNb71p20DVsmMzx4P+psa5uzzwv329CrA6076BKmE
/// 1AxX58g+nT2RSMhm5opFNVsDagv5F3LUV9YcaDrv94OQDSmmAmrsvn31LhXiyzHBcR4B/4zgib08
/// Q9i3ReAuiLV7xUkK8i5QSbr3MC4yuEY6l/1DlLCNkK/wmpD7UL77E7BF7fWrCHkeQsgCMCgw6Xz3
/// nBY5SetT78G4lFeiqbUf+7ufg1Gp8Ql32d2nC5S1ICdkVhrzZ//w0prcj5oraw900n8xiTjCVECt
/// 06up2ELIdQS0ZcLcWm+3aF8ErIdKzU6NHxP/G5iks28T868MvpH0QywK7jPJm01G4sgLXTKeisk3
/// wAdCrgkJCYOelgS9b/1p9bBb61PvwaRKp8C6fo4/Q8MF+IvdaxjyUbV8A2FSOlsVMPenO0R7Zd0B
/// MmkvITezTAXUWrEthN8x0XEXAeXD/Qu836J9ESDUfSUXrmE2j4JIRTjbHJo2gjyPYTJERWxNwZ5V
/// bJ3BaBmRxLOkZArkihgmbdLQ02Cn891z2m22mlHtU+/BUN0+TK5e1xhXs5lK6hvQLt5o0kKrS/Cr
/// Mh+l5qnrJZj8vtJnIuumvrLuAJnP/EtpNRVQq6WSCLGFkLMIaM+CObWEcwj4CGnq329y/0fMz5al
/// ZMU+5vc1ZJCl+czOaLY1ffpNts5gMyFZmeoc+ozcDCZvQGKhZz5One+e07azOXfWZ4+HXgj48OGD
/// NttduZyadKdfP6ynsJOQpRSbIPnHperi2VY28Zdrih/qK+sOkDUzyYkKkwG1QV152ELIUQS8iOJC
/// FqA/AvI1WW2S9gka2Ix1HRE+I+RHCv9ml+AVIefpI93dJEjeSErgEyGJRWyKXKVdSe2opkaOSK+R
/// Wp3vntMmjWUOsT57PBjVs9kw94UR9yx4yJZHrhBSp84tMOlSdFChJNmUugWBvbL+ANkTXXZd62gk
/// oDYJWwg5iYD2DZDSQLiIgAR4y377di5R+r8jyrRi5st6qLtiUQ08LuJf6DibG/LoclcDvH9ONkQz
/// 5wUXPWA/swk31Et0KSfTV1Xkq5gp8+t960/rpLawlQOMzx4PRlQxnlppdG5vVRSbvlWC6UzofHNV
/// P8TfyRcouUpInLpsoL6y7gAhp+kTOkcjAbVRX1fBmEeY9jiFgNeigBtcsWgvBLTtAhDO+XmqD+N6
/// P/VCzhvG7VpoUEr9j+DdpChIICbRaS2kTZhDyLA/mGPRu5jNq92JwFc3HipPTwoZmcz2D9T71p9W
/// DmVE49PAQ3+d3W689FW/FGBTPSH7gCHJrSETdjKf/n/5a5WklVKXAzRX1h4gpPiM3tFYQG1VVTLM
/// eoOpjzsIyKOmcWe5qUGPFCzMZUrvM3iWysvH/ZW2ebC7BnXfl3GQWoPpjxsI+JJAZXOo6DdoBPiw
/// zQBNYKkH5egsGz3YXYO8rySSxsYBTiBAEih6zSWLDhoB49mMf2mK2XOSW14LWm3x4AgN9r5dBfwg
/// cRemQS9HgGI5bOLW6vODRsDn344V5ueaz17v/N+GGps8OEKDv688ix6OIwe8GwFvhgff55hFcdYg
/// m0qJ6TARRw54MQKK6CmcW1gGEWCbpAmQLEMzeCcCOtdCJvfmkkcE2KpiEbVNjmbwQgR8jPG7zUGL
/// IgJslvJCMP8cLjzidQi47Rf7mSACUNZIkU2NKEUzeBUClJtgKzfbexABA5IsFZKq0Azeg4AfCTyu
/// pgREwAD1JIbKxCoBb0HAG6GAs6vIIAIGKlVRcGAeVgl4BQIkvCkNBBGAslXyLEp0H83g+QgQQzqH
/// WY4IwCoBjiNA8Qt1icsWRQQMTo9GUn+0oxk8GAFfRoW9JIgA1IClzPeLKEYzeCwC/h0SU0sQAajB
/// qD4Nkj+jGTwTAWW+iVwfAu5aBJhbq1XpOeNxno+is3GOUU9EQBG1gvONOnZFwLMZANMS45f2qSH7
/// M2vaFCNrg5CTUynNzlPwjZkI9MRoGnRtM/I/eOBBpYE8n6gHmDw9DgHZkIkWtW8uoA4iCWlbQvXq
/// PZvP625ZaKzjRWeQNpWXTmtnQiIipDmqJ0cdDJ5kx29MaeALRiePQoByLXURDWrvggCbjkkNzDJ0
/// E40wdbZIm8pvPdB5za3qd9BT9ETEy8ZJhTwIAW0zfB6iPR2DgCaYZOjGE1lCQEe3zmun0mMRQDr/
/// R8fgfCIegwB5PP81mtNBCDgHB9gUIV4dm/QfKc0A/4z00q3Cbwnhcq0bkU7ceIBqY1J5Y0rgKKmB
/// V0Jad+/ZPnO7XIOAU9SOct2F7qYL5CuDot25K/ennyBdgXHKIxDQPCHsP7SmIxAgrChZQ61iq8fX
/// fiBkZohCnba7XtKQ83hNm85teCAhixuZVJ5dUwpxhghQDD9ISOPwyBYWAfK0tz0XquPBsS9XYaJb
/// m1PCjyjDSOUBCKiPEX5CYzoEAUHZtJ96qtAKUKtUm7ZHsEsK6t34kE+qFJq8fhBtiIB9UM9sr8Bu
/// 5uDn1T8MLzSCPTuEcm97NqRBajNGK3dHQE2UqBZt6aiCwGVYwO6dje5VOFCndr3bLR+IrdA66sr8
/// GgQkQBv7hmAy4y5a0utCvc52X5UMCb2D8cq9EfBeMK4RTem4uoCV6uVFD9HqrvMqQwT0uFXPAqrI
/// GAKmq5cn7oS5jHspiA09eQgCiDwdUn9gzHJjBFTyJ+OqMI5EQPsoNvt/E/7H/H+Xb4gAvRvzcx0E
/// xhBwUJ3sZZDLuu+D+waePAUBhNwXBGO/a/dFwFt+Uhsa0kEIqIUANrny/D6Szkj4/dr+mQoiZ7sL
/// ESGbwde70XKi9JtIhoCCkDB11p+0QTj70xEtqCdkS7yS8UG6p/tX9ngSsql/KHhCh86WdEj+hpHL
/// PRHwMSweCeAoBFT8DpDxlq3NCysg2N7KtQAAIABJREFUNfMCQ9N/kP8yAA69OgyQXkl0bgTGiZcn
/// fz4JsLXtBMCOLkIe/cZ4fcZcQ7F75o7dh7vkRwCOf7sCvjktWk/nAI625gHs8Yje+A8j+Ncxdrkj
/// AmRD4rDh1oEFAZROigz4FQuc7oeAL8KxOOUjIsA5ejQk/Dlawc0QUBs5GttsEQHOUuM8yMRRA26F
/// gAZRDDbWIAKcqAv0eBlawX0Q0DRS1IAmRAQ4Ux/G+d5AK7gLAjriIrCdBhHgZHVmQhpOL+oeCOhe
/// GPQeDYgIcLqKA0biPONugYBNNI7lRgS4Qp8n8P5GK7geAYfgNpoPEeASdW2FrbjymKsRcAnOoPUQ
/// Aa7Sdd7kerSCSxFQSv0PjYcIcJ3ei4a+RCu4EAGveb+j7RABrpR8DlWIVnAZAhrCk7AohghwrVQH
/// YEs3msE1CFAmCLFTICLA5ZLQM3GAimsQsI6HU4UiAtxAlYLhH9EKLkDAWYzliAD3UO1Y/gu0gtMR
/// UE4dRrshAtxDbQsotLezEVDDX4BmQwS4jXbBCTSCUxHQMSYGZwlCBLiR8iBThVZwIgLW+Vej1RAB
/// 7iQJndKJVnAaAiSAczgiAtxMZb4zcfiwsxBQ7bcBbYYIcDe95k9pRSs4BQHKSTEdaDNEgNvp/dBY
/// 7KzmFARs4+EkIYgAd1RNpAhnsHICAkqhCC2GCHBLfYkajivbOhwBtfxVaDBzSvTh8XhAM5sAjI7O
/// Vv3IKMwHOBoBSSJcOcysToFOsWgMp6th5PDvaAWHIuACVKC9zOqbjgAUzqjkCgaIRuBMQo5EQI3P
/// HjSXBU3WMQCXV3CF6obhyjaORMAsEXbBsqQCLQES0RQu0VfhWFx41GEIKACcMtyi5JQGAdhw4iLV
/// DJ2KHVcchIA6/yw0lmX9pKkKwE+Rq1QVmIxT2jkGAbNHIF2t0DU1AX5BQ7hMFbxf0QiOQMAleIW2
/// skLtNMuAW2gI16mM2oxGsD8CfgRuR1NZpcUUAI0ZJlfqKuSiEeyOgDWhOBDLOpUw5YAVaAaX6iQU
/// oxHsjAAp4JLuVkrpC/AIzeBabaCxE5t9EaCaMBUNZa3SIQCrpF2s7uSwL2gFeyLgMoXLuVsheU11
/// 9SdpASySvq+urm7qQou4TIqYUdgua0cEdAo2op1MZPs/lUvy966aM2nkUB/oKzo4KjZp2baTV8r+
/// wwlXnaya0Lk4paj9EJDDG8Dgi7pHfw483kslbt/V+1PxyXVJQrYzYGjMTyu3H8y9JCn7RyqVyqr/
/// 96m6ktkpKy46cyxr7fxJAvak4LhfD159jT2snaYKat/gL/L1/jntxqVycFqyjAB5YK9lxOcfJrkJ
/// hDyaDrDOzBRC0rhQQsqEpuBxL2XCnJEApUY/rSuOvYLePrU3NS/tSe/WA/yyfEYM7LPSg43qenk2
/// fRIPYGjC78duSL9ZUfBvqrqXu+mn4RRQIxcfL23G9OkMFVk7dYs2FrxZBL5rF62QGR66M3SmdmN9
/// pBpAjHVxWrKMgN18wwbBLnhBEvYyOw9gmVlv8WnMswnrjB5rTBQ+JuwE8EbneBAPI92bevnU39Rs
/// 0tSdJIMZ7P/cYms9WK/G4l3xNAQkbCqssJ3Lync39/4sBBD9fhmnX3O81vOsqsHSx4JbkEXIk6he
/// B0NydBurIlXbAGOsa9OSRQT84J0y/PtZREgUu5JAjnHs6NROXTGdkIZFNKp3Jhv16ddvNIL+pnrd
/// LOoyedJ1TdeQZ5bmkOl/VbPqlorjASLTCgZXN9palp1EQ0hqEeYGHFxPMzXKmmWH9bFgJ0iZtwOG
/// s458ZGfIUG+silRF1wYYY12bliwiYH9Qr+/dC8YSP7M70yizUwg9BNMzuCTqBh2+NJ6Dk/Z10t9U
/// r9KgoacUJk5aBe8IsSKd9r+qGb7fSPGFqI0l9qlnVr48FAcQf/orJlQH6vuQZJUNsWCMkI22AYbH
/// Cn26tRtrItU9qmGAMda1ackSAlr9j/b6X9lEOtibKqjpOqf3xy6lqClWmndJvGI9a7pruanRpCUH
/// jhDZtviTVeNGsyVmZX7WqTGQQErgJ4MLanzrT9umrkxPZnz2eNDf1DBN5oX77ak3FjIyJJwJ9lpm
/// p2ZrQG0h/2/tjz6guv9GrmpcnXcW86iZ5z7ZNYLKJasCIP5PnFvEcXpJ5Vg+SRcLGmCT8sft4DOG
/// MXpFsm6jj1RN5/1+ELIhhXH6dPZEokH83qweJP532g6qwsYY6+K0ZAkBx3yN56bugli7V5ykIO8C
/// laR7j5gtMl0jncv+IUrYRshXeE3IfSjf/QnY0sj6VYQ8DyFkgeFwep3vntMiJ2l96j2Y+JReiabW
/// Glld/i1ELEnwO6/OwQwvrcn9qPvR3Ur330rVZgdDfJ4jZqTqLkvzheQyTKuOUi5l/RwXf7HtNlNv
/// GMZJIjxNtJueSNVJ/0XIuwhCni5Q1oLcIOLOYkrshF9O5sltjLEuTksWENAZbKJOYz1UanZq/GQs
/// QuvIvk3MvzL4RtIPscG/zwSJvaU48kKXjMfkyBTwgZBrTG4rDHpqw/S+9afVw26tT70HkyqdAuv6
/// OZ4CSbcsWd07bO5Pd3p+9LfSOVulh7Nh6ME6h0XSjitxIDqDw4ocpF8EVrcup8HXvnGylo3j6o1h
/// pJrEpIibWaRhyEfV8g0G8buTZqsCojaSSltjrIvTkgUEFFEmor9QR5SFa5jNoyBSEc42HKSNIM9j
/// ull/TOlmDzvn+LTRMiKJZzNCTFFJEcMglIYm/WV0vntOu81Wjah96j30+nKyXXB0JfKazVRSv5DN
/// YNBMCtXU9Zlo8KO/lfa/VQCIgxl3HNzft3IdHZaHEHCI5MKfrD01dES/OPl3oEq7MYhUZD4TN1Na
/// yarMR6l5hvH7IbC4udRvjl3LMdZVack6BMQuN+7+EdLUv9/k/uygmC1LyQq2xfQ1ZJCl+czOaLbl
/// cfpNtpyzmZCsTPbcczNyM+6z14SeahWd757TtrPGZn32eOhl0A8fPsi0aWc5NelOvwqfDnqCNttF
/// yjW5JO2P7la6/5b1Kg7mOGO8ScN2OuwidmZzhCqok1aCGDb1jZMkYwHRbgwiFVkzk5xgYgX/uLSb
/// GMbvrZoPS3ysrTHWZWnJKgSUm5o3PB80REn7xE6W2zCkjgifEfIjhX+zS/CKkPP0ke5uEiRvJCXw
/// iZDEIpaCq7TXOqqunyBEeo3JZWl995w2aSxziPXZ48Gons2GuS+MuD+AXZra3Jckm1JXwWp+9LfS
/// OVtSczpMd9aAs4Yt1MR/McE6QCco697hCf0A456IMoqtG1RvDCIV2RNddp2QOvWHvdMg4ooOKpSl
/// cvLS5OzRpmKsq9OSeQQsnmDiQAK8ZbZdO5co/d8RZRpjPv6h7opFNfC4iH+h42xuyKPLXQ3w/jnZ
/// EM2cF1z0gGVYwg11flc5mb6qIl/FTDlF71t/Wie1hS3QMD57PBij+3hqpfEmmrXqOXuUp2MIidM0
/// fmh+9LfSOVvQLX7oNSdG1bfxsANHGDpAs6KsWv9mEujWf9JHFDlUPW1VbwwjFTlNn2Cjvm+u6of4
/// e0/E/QIlV0n2JVLn22VjjHV1WjKLgEbKeJa5bReAcM7PU31AQu6nXsh5w7hdCw1Kqf8RvJsUBQnE
/// JDqthbQJcwgZ9gdzLJpl6KvdicBXN3goT08KGZnM9mnS+9afVg5lROPTwEN/nd1uYsWuHBoily9K
/// 4IGYtGpCr/3R36qVsqIcoNwBa5081qyQF1eHKdbu+haUYUWdwXaAjbpeP7qI0hb+oEGzMYhUhBRr
/// lom5NWTCToVBxP2Xv1ZJ0sJy1z2wLca6Pi2ZRcBp33b7xfBcpiw0g2dppMxxf6VtHhyi5gT6itNv
/// WiUKwXna7a87cM/bHsm+acksAmLW2S/YPmzVZRNY6hY7OstGD46QPDbijQtuq5jv8w8mWbtrBd/b
/// +l/ZNy2ZQ0CFPRcRHM9mVkpTzJ6T3PJa0GqLB8eoZbyg2iWvVvmLL854Zf/XKZzvZU9k37RkDgEZ
/// I+0Y7M+/HSvMzzWfd9n5vw01NnlwjJaFyoy6P5sBMC0xfqnJruITd1nvalRdcwRyTLP21jNvW93J
/// vmnJDAKUwcc5Wng00YBK6iCSkLYlVK96FYNaniUHjPky7mqiCCJYjknW7toWgKuOm5YZBDwCGRct
/// ouCbrgEBEbOpgVkGTjVT7Hr3B/AAI6W91R71MxphIAhYE8tJi5z2aTaPgCaYZJAxGCWy7+3n4mzN
/// jigK3EQj2I4AZZCYkxaJXUvMI+AcsDn7TvHq2KT/yFHwz1A92yr8lhDeLFk5Ve8uCYT9hJyHQhXr
/// ejddIF8ZFM0O3j7z63p2MkGTtyiFGoyVdlc6H0dl246AMvjMRYO0mluKBoQVJWuoVWy9ytoPhMwM
/// UbBU6HpJQ87jNW1fWULo3M+wOfqvy9jBmiJSx4NjX67CRIYA0ExyYIfpW3RRVzBW2v+tYh3LABCw
/// OYaTBnltrgYEgrJpP/VXukIzUXipJmMwAuTaTILeXRk+j5ADldqswwj2sx9CETIPlKTKsCTRTzH7
/// MVbaX6Xm5+ZCBBhT1F5OGuQ5NJgtCFwGdugYORttWDYQgW5f705OQ7Vyke4M9Qns5gyTyfgE5kwb
/// n4mx0gFaKmxHI9iGgHfAzZ5qlezkCubqAlaqB2cdotUxStUXAXp30uqzTSLphwByKXDn/MPdZgIw
/// 8n8YKx2gev99aATbEHAisJuTBmk3V3nMpub2UWye8iawKfVdPoGoXgjQuxOyzS9F2Q8B3VtlFu5P
/// 4RKuDlE+9Q6NYBMCEpdx1CKTfzV5qBbYCWbf8fw+ks5I+P3a/pkKEsX7SogQ2BGpChjS4872H1DP
/// vMq6Micwe0NBSQ4Pu/zwpcwMXSVQj7HSEcLFcW1EQBd9maMWKaRNzRVf8TtAxltCrkBYAamZFxia
/// /oOQP8Jutx8GSK8k7X8A5Cp07oy2sh0M1K5igKOteQB7OstC2NpC/m2Tt586ByOlYySFq2gEGxDw
/// jJtNgow6wxc58OqXTzDfo9orwaaO/w04YNhRWheCAzBsQMDBcM6a5DE4brogsWbA5uexJo5/C8T2
/// AIdJHuw2xm31AAQkrHJaEC6PGj0U4FlvRxuG19ldm/2kjrp0MhxrIuT1IhODDRWTRNh05cAyHvWf
/// c29YPgtgxrQpm/q0M5+cSml2MoOASp4VO6vvCqh/Zk2b8tHFCOiinTa+8jLcIKTYj51C0eKgO+eo
/// K9nfUQxo3hxJx6UWmpglsD0hWFdr3Si9hXMK21uq2CQn3/Eb22DUkGgYn5hI3hmk6yBeD8OZ+LYV
/// eq3bSfJ53S0LpS5GQAU4DUIJwOaKbubYfdDdIKoDZvCfueC29ZODqxoqboo3zB5GAURgkrX/V9np
/// fQTV3UaqYKHeQR3JRdDruJKO7OVJNMINCgJ5gU4LwVQ4yGy7i+0/6G7gak+hnD5GSrqOpsPZEUQ0
/// re5jvARTrP2VEtXpAgQoQL94jSaS90EACeb38sQTuQECljivaUoCME9dVuo76M6G4XX212lqrnPX
/// /e08AL1E5WOCtb++0rkuQMAT2E5kKVlpU96qIzmLgMaUwFFS3XEJu/BXIRO/FaeZTWkGc05Gm6sR
/// IDzsPCv95Q+B6sV0+gy6s2F4nSPyjFG8k06c2P/JCF5u7UJDBuAsgo7QXv9G5yJguKq5OML3A4ka
/// RpT+0foxJdk1pRDHHvdb9WtcgHqpsmGg24Ab5AJ+wEMnhqFpA0ByG+k76M6W4XWO+CwfpKOLnXSv
/// qkUwn12zsmQopUeA+CUuLWJ/KYZsci4CmIJd+BoZIbnXmfRNGQ4rC6LZ41Ff3petp3eoDMeRuAMC
/// HoNzZ1ioDIcN/Qbd2TK8ziGSpcAYZ0CgagnEaOe6b8+mtBAI5QM9dW8pdmaxswopmdMLAmq1nTsi
/// AMNIbpDaz7CLlLgXAk6HOi0Az9Vr6X2GQDMIsDy8zkF6kwKjzjm2G4fq/s8QI+lpAPyYoKkK2Eqq
/// izJjASLTCqow4dpP3SMXuQQBUuEDdVw2ioAGtlDgXghYMctpAfg3QR39Q0Wk76A7G4bXOU5vV9O8
/// da8ddvl6cSRM77PYbFEgmxG4rt5vLctOoiE0WVzehanXPiqGV65AgEhAyHAwjOQGqf0N22ooAuYd
/// h7kLAkbvcl7hDFYpCLkHl0nfQXc2DK9zpFoLRkJIpiM6ajQXJVO+6f07rDWvYRDQM0Sj4/mxOf7g
/// M1ss7cYEbAfFJzjvXl/03Tv8oOxaMFTUqiP5EFCwqb2NdALbEb8hjmLi1wI4IMsNhIcqOUS6HAHd
/// tBPXXgiDoBkz4tgyd59BdzYMr3Og2v6eT1GTBDBq73O7Vs+9z5tB0akS48u9vhwZ1Ke48N+FxSHg
/// l5z7L/YZHKwqTK4TYf9bLQLYqGnaOec3oSI/YH4zE8nJSYCtbScAdlxPAZg4O06kXp1GNpE3UzYl
/// 7cbrDIBDb1yMgGo3a5GyNLzOYeosSeNBfN4PoirfPAx8F1x8b5fLNtxOj4CA1Bum236VZcZeS0Eq
/// H3ySxFLEwKA0PwYNaAkB98GtKqItDa9zkLpKV/lBwoUfuv+ysz/zICj52DPFIC6qfHMhLQpgwoF/
/// BparZzAQCPxksRSj7oD1DidptoiAvGC3CqOF4XUOUced5X4wMbeuTwKW5i0WAAxbePD2J5uTcMPj
/// 3N/G0cxn/H8PBkXYbunJOT4QtuwKTo0/QK2Kwi4XFhCwfopbhdH88DoHqL0kzQfixTLjR79uEi5g
/// PuRU1OwNp4sr6iwGq+G/hxd3pYzxAQhO2lH0xi51espXxxMpGLf3BcblAaiGuoxGMI+Amb9x2CKN
/// l5MpasZF05/YQsgiRFFx9dCK+DC2mjI4Ztai9F1H8oskkodlZc+kL8vKym5Jrl4Q79nw67wJAraF
/// z3dsyu6Cx3aeFLC9LEsEvKS8LxiLbdXaCGxjNY+AEYe4ao4qcRJFJeWZy2JfUI9s1JUXqstv/7n/
/// 95Sk2OGhdO9hPoGRYxPm/br99F9PqhxXs1JdkOoDkeklnRiTbdEX6gIawSwC6CIumqKjLFMA/DSJ
/// +d6AeXDC9EG5XP69+guzdWL+vPPRNiYzkHz2E0Zm67VxSAcawQwCGuAp5+zw8dwcGsYftFjRfskc
/// AVym+qJUf4jMLMfGLiv1ncax2OYQIAVufVHqr/4WDj4LL1tRVr9NuetaP8onW4QQurYUiwRWaVso
/// TtJoBgHFwJ141FaWFQtUbFaZVfVDJdQ2d36YN0digZfytwKjtUU18HLRCKYRUOjHkcz/X5vGUzB6
/// x31rZ2h5Ra9x92equzCDon+52oox22I2AGsDTCPg+DAvf2h5ZfHJ38bzgJqw+YYNk8hUB8/1hGb4
/// pkuzKXreX0gB86U/+jwawSQCdkz0ykdVNb4rv3FiY3K0DwAMSdpaWGFbeadpeGybhzyqvCiZopOL
/// sLhrRuvDsW+ASQSsnOsl5fyGT9KnJX8XiLN+nx8v4qub6wWTf91X+PD9ADKBnXGRntQjV35pGgSs
/// fYZtBKb0BbsImkbA3JWeFNWbqmXSirL7kr8K8sXZWZvTl6f+lDQhNioiQNdHJyBy3KzlmYfPSZ69
/// /TaIjPzv/h887N3WnhwDgj0fMZIb12/DsHe1KQRM3OFuoWxrrv4gLS8rlhReEB/Oykxfmpo0LTY6
/// MiTAsDteQEikKDY+6efUlenbso6eKLhe+ux1daO9GjfOwj0PfL1VewUwpagN47kRfaL+RiOYQEDU
/// cVeHqulDeXHRn0ez0pfMmTJaaJDQ/YOYZD45aUHq2oysA+L8gr8lZWXS19U1ckdXfr2kDnvmC1aV
/// pdG+aWUY0/tr2UgsJplAgH+BK0LSKXt2Nf/A+pSEUeo5goAOHR6btHBV5n5xwY37T6Rvq7/LXZdx
/// awid77nvWF4wBkR5mBXol0WCu2gEowhQgjNn6FJ+k0ryslLjI9lkHzAyPjktK6+opLzavXonzR7m
/// 2W1sFSuowJ2fMbr31s+T0QZGEdAITllTU/mh5HRGYgRbig+fvHRXfrG03m2tdJp66ekvuvGYAH7G
/// 8kAvvYB/0AjGEPDV4TMHfrmXs2w0BRA2dbVY8uqb25fI3tLHvOBVK28lwDgJFn8NFPcL2sAYAj7B
/// v4673/eS/80JBgj/afeV157Sl71TlOAlCacyDSLzcCCRXrfhHRrBCAKqwEGT9H0u+m0YQNTS089b
/// PMpG+3y+es37/vg7NRQhoJNKtBaNYAQBlSCz/31ab60WAD31wCPPWyTvP+pPb3rjtVvpiCJclESj
/// i9R3REB/pwqosfNNak5OoyDu8AuP/Pqo4sd7WYKpS6dEEkz+6jJe6B+IgP5OL8CuZGw4Gw+By679
/// 8FQLnaPeeN1b/5AC8ZUIAEZHAhSIgH56rFm5wz6f0IfzKN6yex48JquFv9Mb37s0ntqGU4sQ0kSf
/// QQT0032wV9yQn46ChL89e8hqVkCzd775Iv6QIkQAyYhSIQL6qhjs89FWiP190j09E11He+0EU40r
/// YEET5xHwEUoQAX0lAXtcuO1oQMBhz5+5ZqXQi1vQngoE5ZxnwOxEREBf3QI71ICXCnlZcs83jwyu
/// evPbb0mlsrmeD34IlYiAPioZ/ATC31Jgab03mGeD0Munlcil5nB9CGHM74iAPnoAg82/PwuJfOAV
/// 1mnm/entEaCCHyfndhK4SDUgAnrrCQyykiiPWuIlX5bDHGg1/hA+so7TSaAj6BAioLfKB9c1SLmc
/// Ou0lxukK3cuBKPBVFFXP6TTwR2gXIqB31hAGs1x15888rxmVfp3ixPexccRYTi86UMvtSQSNIODf
/// wQwTapvh7z3TMCRwZDj557BpnB48mBKPCOilqkEMom6NC/aeJpYP8IAjseBfH07Xij8FKSLAUB8H
/// 3lDamRD23ntssy2cM2NqH0IelxkQvQYRYKiaATOxO8XPi7pZdAQe5048OE5xeV7Bs3QzIsBAP+Dp
/// AC+2hn7hRaYp4tR0EouDOdw6rvA9jQgwkBLuDOxa/6NKvck0cYvcO3xf75/TbuySCqJmuUU4XKON
/// w1SIAAP5DGy9xVtQ2Mdl/mGSm0BIxSzwy1i9zlSHozJhvZWOPZc0L+1J79YD/LJ8Rgzss9JDL72B
/// J+7zKG8Wge/aRStkhofuDJ2p3Vj/oGZuLaVy3SIcpk5yqOnfcabm1zoECE4N5EpveJv6uHTBC5LA
/// dq65Asw2p3cLW0//wUdCI63vRh0NLmlO+pNkMIP9n1tsrQdDZUa506PcgixCnkT1OhiSo9tY9aBt
/// Fm59mKp0i3C4JBZNW4AIMFDM/gFcqHnY5L59rD6LCImqZrNZUM58ZXx6FbSvDSi4+kvqdbOoy+RJ
/// 10H9YXv2zearMsUh/jF3epSdbB1ta69+mx/Z9R7UG6se1CCYxp6XENW0ke0WA+vgcJgzgSNNT65Q
/// jYiAHiVsHMCF5kT0s+ELJj78zO5E+SsJkYwwOHRvgEMz9JfUqzRo6CmFiZNWsT0cqgZyVXbilC/u
/// 9ChjhMzmYYDhsUKfbu3Gmgc1DKax52X0JWCDxcA6OBzmTOBI05MOvzxEgEFJaZnt18mn+ncKrGwi
/// HeyUFJ8glZCa4eppqj6dPZFIyGYAKKrZGlBbyL+QA0f0B5rO+/0gZENKC+so2xZ/smrcaHa0rjI/
/// 69QYSOi5pGFeMC/cb0+9sfuSIeHMF4udKV5zq7+1P4S8P3YppVTvbOyq5JdEd3qUBtik/HE7WD3P
/// nTbwZEWybqN/UN1NdYHoua86mOTvtB1UhdFbq3UVXpl4vU4Lh2kTONb0ZHUMIqBHq+bYfJkq2sxo
/// q3MwKjU+Qb2K69MFylqQEzIrjfmzf3hpTe7Hr/C650An/Rch7yKI2vE+lO/+BGxpbv0qQp6HmLq+
/// 8ko0tfZjf/e3ELEkwe880d9K91OcpCDvApV6ZyNqoorc6VH+YmJ78NQb6uyJLvBCtiFLvel5UN1N
/// dYHoua86mPxyMs/c0ODp4yx0hnJCOEyZwMGmL+fszCHGELAtztardMbEm4k7P0PDBfiL3WsY8lG1
/// fAMbQ9hC3Nyf2NbH4hCDA2TSXqZAmKVxFEde6JLxVIQo4AMh14RmAlA6Bdb1czwFkm5Zsjo/r7mV
/// 9qfGT8Z+z+r0zkb0J0/hTo+SBroFjfSBr2XjrHpj+KDam+oCob+vJphRG4nZiP6ROmv+RTsnHEZN
/// 4GjTj9iCCNDrkMjWq+z2M7P6SBdvNGmhNYXzzEepbJnrITA5NaXPRNZtzyqDA2Q+8y+lVeM4bbSM
/// SNgRHG1MmU8R81fvy3b7MF8k3cpkNZuppH43nsF+KdQNldpbaX8Wst1BHwXp/xtT7Eq3epRQfRFY
/// H/i/A1XajcGD6m6qC4T+vupgkkuWVozN8jXfHcph4bBoAgeZvkdHgjoRAfosV5CNF3lDXTRz9Cns
/// JGQpxb5f/nGpOrewdaI666XOa0+/aXCArJlJTlRoHBXUZiZOZqpDNCM3437fSPPhwweZtri4nJp0
/// p1/fjg56gjZrqbuV9sf/EbPZslT/31jBxngPSVc9SiXoG1z1gc9gm7HUG4MH1d1UG4ie+27VwC4+
/// 1vybbBeuNHfYceGwaAIHmb5HX+E2IqCnMtw2HnbHTjLXtyoLHhJSBlcIqVNznrm46KBCSbIpdV47
/// SN7Yc4DsiS67rnUsgU+EJBaxR1aZ+3g9mw1zjXVMfgC7NDXWL3W30vzUQgObZ6zT/Tem/QKVOz3K
/// CdC1tPcEfhRbJ6feGDyo9qa6QPTclw1mqZy8BAt16LfB3JzCTgiHKRM42vRMdmExIkAnKXy26Rqn
/// KHPtQKoo9s2oBNOZzJxvruqH+Dv5AiVXCYlT5+oa4P1z/QFCTtMndI4bopm94KIHLMkTbnSYuHzF
/// eGql8duvhVvsF+l0jO5W2h+l/zuiTCvW/zemkVvc6lEmQa2uskwXeDlUPW1VbwwfVHtTXSD091UH
/// M/sSqfO1NEHODHMVQQ4Ph0kJIvMfAAAgAElEQVQTONj0rM7wOhEBWn0Dm2aX/8Y7aOZo/VKATfWE
/// 7AMmVd0aMmEnA+1/+WuVpFVT594mZLuVaQ8wOZAzesdh7IqP0eyH5dXuROCbGH9wdnut8QM5NEQu
/// X5TAA7HuVtofcj/1Qs4bg/9GqsXgmRs9inw7wEZdbxtd4NvCHzRoNgYPqrupLhD6+6qDmRaWu+6B
/// pbcpBZN5ZSeEw5QJHG16VnVQigjQZezhpi2XWCVw8JJhhblMAXGGMxmdw+/2lkexWT/HemdMt2z6
/// 8emIAJ2G2LKIViXccHAYfdihIU3gxCHdCSu85lFs1lu465Ux3bLpD0QgAvQ4tGUx3cRJjg7j+MfM
/// pjTFeUZpp654y6MMQAtjvHLgrGXTP7OxDsybEWBLD+ESk51K7abPvx0rzM91Yub5AdR5y6MMQFUg
/// 8caYbtn0XbxCRIBWGxKsv8B4LxxlmTWScFlLuNpdfkYaIkCro1FW+y/2xq7V01dzGgGVvSZL4ZD+
/// GIUI0OoKbXVpMDbFC43if57TCCBT53HzuSXQjgjQ6B/9YBDLmYA33mcTmaWe9N6uW2yPOg6Kmy/e
/// KAIarJ5DOM4b19u5Bxxfb7tbuI2Tz62iriICtPK1smr0H9v6EXqI/uQTjkvsw81FBtV9DBEBrMZk
/// Wec7xSt7ku0Yz3UEyHlnOPncCRsQAbqkbV0lX43DOwa6REsWcB0BZA032wUXpyACtMoaY93nMlzp
/// jTaZu5LzCODoNFqaGRARAYwKedb47eIf8UqbTFvPeQSQYTu4+NRrZyACtHoK9Vb4vUbVeaVNJm5H
/// BPwvRMnBp7alW6yXI+CrVTX9CV5aZp66EREg4+QKW7/+jAjQSmXNgIkPpieX8Gz9/Kur7myuKU7p
/// 3PbXuGUcfH5O1gIZRwCZYMWMyvsEDh9UennU6KHQdwafibs85VvwbAbAtMT4pX2mwvoza9oUY4sX
/// nJxK6cphvjETgZ4YTeun2JP/wQOnRoyLtIJ7zx+/BRGg0++Jlr0KdzmcAGyjY7EfO++zwXxSSw44
/// +LY77NbboQ4iCWlb0nvR9Xxed8tCqZGzO4O0sbx0WjshICKkOapnBHuwcxHQRN3g3vMPPYUI0Ol0
/// iEWfTmg3SgA2Z3gzh5CaKU60SSFtt+wNG49JDcwydBONMHW2SBvLbz3Qec2t6nfQWUpYxrnnl7Oz
/// FCMCNHoEFhda3SByeOCmAjsxaXcxqRslcqJN/oEauyKgCXpNrMQTWUoCHd06r51KlyEg16+La8//
/// HGoRATp9szhQqLvX4tuOkQRgnroweBT8M1TPtgq/JYQ3S1ZOJXfTBfKVQdFsbvLMr+spALtGjxb9
/// bPn2SQLngC26dIpXxyb9R0ozmGdJL1U/i1zrRqQTNx6g2phY3pgSOEpq4JWQ1t17ts/cLtckgVPU
/// jnLdhQxs4BDVwGOuPX9uEEEE6BX0pwWPT+Gj40P3lz8EXlRp4kPXSxpyHq9p+8rs1/Hg2JerMJEh
/// ADSTHLBzP5axm+2GAGFFyRpqFTtd1doPhMwMUfR6Fp3b8EBCFjcysTy7phTiDJOAYjiTEWocHtnC
/// JgF52tueC/XYwEGKyeTa889ahAgwyIOvs+Bxc7RTKqU2ACS3aePDCJDr4sYI9rMfQhEyD5SkCuw8
/// g2nWCLshICib1qy3WAFqlRo+i96ND/mkSqHJ6wbRhklgn7qT1hXYzRz8vPqH4YX0NnCQ9kVw7Pk7
/// 6cuIAIOSfrwFj+H7nRPAynDYoI0P2tIgu6/eZTdnmEz7J9hr33s+sdtUskxYL4O6B9XZ6F6ZY/UD
/// 6N1u+UBshcFT9SSBBPXcBTUwmXEXLel1oV5nO0BSO0wH41HP/4CTVQEmEXDez1L8eO342pl/2e1n
/// CDSDAHIpcOf8w3Ze+KOLf9SOdQErgR1KcYhWT0qlMnyWHrfqWUAVGUsC04GtE++EuYx7qXqdHr0n
/// RyOABJ/k1vMvm0QQAT2qAJlZfwcFjg/bvwnqxrlQJi5AlAkEdG+VOeDOW4V2RED7KDb7exP+x/x/
/// l2+YBPRuzM91EBhLAgfV0V4Guaz7PrY/pt6TwxGwaA6nnl9OFyICDMtF1HWz/iatdXzYFLBKwU7j
/// xZTQonhfCRGqM4UKGMLuMntDQUkOD7v88KXM3st//Wd0TUHbVQsBbHTl+X0knZHw+7X9MxVEznaX
/// 0TyL3o2WE6XfRDIEmOcN00xb1gbh7E9HtIApDG+JV7KP3D3dv7LHk94GjtIFnpJLz3+G14oIMNQY
/// sysKydWLyDpaYRA0Y0Yc20L3R9jt9sMA6ZWk/Q+AXDHA0dY8gD2dZSFs3RDf3kvDT7LLCKiK3wEy
/// 3rK1WWEFpGZeYGj6D/JfBsChV5pn0bkRGCdenvz5JMDWthMAO7oIefQb45XlkGL3zB27D3fJjwAc
/// /3YFfHNatJ7O6W3gKPN/hH849PzdI9YQRIChVpsdNylxl2XxLp9gioW1V4LtfNlibk6Z0UeCIxx6
/// 2GvwARHQS+d9zPWSXRPnHsEXa1D0eay9Lzx+IRKArJjOnWdVRf9KEAG9JDULxaj97hH8ZDjWRMjr
/// RVX2vvBdJ7R4uL2K6E7OPOt1eI8I6K0uc3OqN7jLeIrmzZF0XGqhA6rE4uIQAdWDrQzwHLWHryKI
/// gD4aZ2Y5idvQ4u2GeUtdQgYE53LlSff6fkcE9NVaM+Nzt4/2fstsDWzkPALmLOXIg8rofIII6KtL
/// tOns9UQOLLnQOmQ55xFwMJIjD5oUo0QE9NM7MDkOs4O6xgHTPICrXEfAff3EXd6tXIrDC8maRgDh
/// mywIPoMvXLDNFp6M4whohlIuPOY7+hBBBBhRcqqpI8eGcsI2nWPGKznOgGHZHHhI5fhYJSLAmHKG
/// mDoyZzE3jPOO5vqKAim/cOAhN/E+EkSAMT03OYUeP48j1rkN+dxGwKFh3v+Ml+E6QQQYVYepzkHf
/// 7TSOzgN0wn7TCHooA9u8/REfUYcIIsCETDX9WTG9sNcog7rL5eghA6mXP+ETegVBBJiSqQ5Ap8I4
/// ZKBN1C0ORw8Vz8un03tGL+tGBJhUsYkRwStncslCm6nLHI4fsVu9+vEk9GKuE8AsAuRwx6j7OG4t
/// vr0fdnI3mvyW6M1PlwfpXG/2NY8AMs7ofPrddBG3bHSNTlZwNX6Iw7332TpXQx5BmUXAzlHGXD96
/// fRVRX70KjZJyNH5IwGunDPg01r8EAWABAaXqdRwM9LXsQye55f0NRX3VMJvKVnEyflR67Uwat/zG
/// fsL0bwkBir4LTFcCQEB4yJbc29ImTplJdZyazcl1JhRwzyufS/4rpHdi8reIADJpXd9qAPViThQF
/// cJRjhnoZ5XuRixEkxCuLyw8FIcWY+K1BwN4o0iL5fV6Pw1TQitfCNUt17KYSZNyLIPGbve+Z6tMg
/// tQnTvlUIKBodxyR3g+nE91EaAlDZHLTV6zHUH5xrGlj5k7c9UfdZ/4i7mPKtQEBbSXooqFO8wXza
/// pdpMAP2Di8ZSFvDDONYgSg5Ee9kDlcdSe9ox4VuDgKWg/eLDbz2OTdpMwB8cNdePdIh7waknvhjg
/// VY8jWwgzqjDZW4eA5hAtASjDPqJCTSaggbMGe50Ic7i00tA98KKyT+NWauR9TPRW1wU8ByPl/pVs
/// 1oDawWWTlY2HZdxZe6rSe9bZku/jhV7ADsE2IIDs0CLAcA7Bi2qXb9w2mkQEi99ypegDj73jQVoP
/// +weJ2zDF24SAzlHq2oBea2q8YR02cd1qqpsxMJ8jfYa9Y0RIQ3YAL0uO6d1GBJAqTYWgxDDys52D
/// atBupGwixEu4MIJQmOP5zyDLpEOymzHS2o4A8qcaAWWGTglApaPZ1BD4CUZc7PD6x5zg8RU/5ckQ
/// ldeBEXZACCCz2XxAr2UW9gN8RrNpc0lraP5ub7fGnJWeXWYrieNIds1BCGgIZBCgGyvWWC4pzE2D
/// 0eKCaw+qVWg71j6HBTC31KttsXKuJ7+fnEhY+Arj6SAQQO4zCPhOSG3R6kksDfyHDoMoYQCTN6Bi
/// Uk7/ixwgyttJEHnUi1dX2jHRY4P+eBEVsEWGcXRwCCDpAPfWRQGVkHn+8VfW4TC7kVcU7V3Ah6CF
/// l1vRhh+28SHxL2/tdJrjoUuLygtGQWwBtgIOHgFSf4Dxex/31KYYFKve5CZT9LIHWNDqLkul6NQy
/// r3y2Ql9PDLU0nfZNr8T0PXgElCdDCO+NmRNaipIgAqtb2Y9OPAiyvHAeGgl4XHHvy3ERjL+EGQA7
/// IODeJEgsUxkut6SSf/lY3dR7uhXZRjrsNNqbkLfbQmDaJW8biP4QPGtqCPmlBAja8Bqjox0QIJsJ
/// ST2VqY23Di4Zx9MNG4hK3ln4Tn+sIdtvSBHakhBlSQpNzSz0qpHUL+GrJ5XI0mgqWdKFUdEOCGjL
/// omJ1HQK6H22JAWrEvF2Ft8oq/pU+vnv10LJYHoQtL9J9IBp/g1kytCajjpI0HsTnffeaB6oCjxlc
/// K80MZkyP8wHZBwEvhf7ntNV8b3aEwZjt9/qOGe2uyJlB04vuaYdelcfQOCm7ngI+TFT0krFUX+Gl
/// Z6T/rEiIFn/F6GcnBJyg5mqnBChPBkGWqfXX2crAcG1loPIY9QuOw9CqXbKYB1NzvSFjJIeHbh9G
/// Vfm2CIjcjS0AdkNA81zqhGbvwUSY8cTsBao30qGnNPWDL4YKpWhRfV7gzjJ/GLHjqacPUO9y9/XV
/// VeWZQ0GYWY5Rzn4I+DpS8I9658sCmGc5Udfv4o3QNIk3zqLvoUkNykrS7FjgJRd4dsVAr5Gibmfi
/// 8swwGJmF6d+uCHgXPlJdolKd0CVtS6r5BZY0ql/IOihEm/bS54JkCmI9OY5S19w1ZM2StAAYmf0O
/// Y5l9EVARFK8u0jckUUetXm+lNGLoc/XOPhCjUftIcWd1GAhWX2/0zOC76Zwh/4knA5WYi8NW7Y6A
/// qsA56uq9J6HDbCnYyxdQx9S9yPLhHFq1v6SH4ikYs/OhB44k8HO/jJ2yPGsEBKUWYfWzAxBQGz5J
/// HUuLqAU22reAWqjONByDm2hWY2ory4oFKjarzMMqCPluxvSvhQt4MGbfKxym6hAE/BDFqFN+HmTZ
/// fKlnftPVwwY3U0/Qrib0/cqKIRCw4JwnLdcb5kb9PdpLt4iAnnsBm/8dhQBVklDdoWUXnBnAtf4N
/// iWX5oVociJMLmlF1Qao/BCeLpR7yGYs44S6Gy0uiITK9BFcEdiACDlOvNHn5gVUCfxJMZusROkdP
/// wE7a5suyFSeT/SHg51MVHlAoiDrmBoFolKQPBX5qQS1GHoci4Dn8qa4HgPwBXq0qMJmN0x99dqFl
/// rcgNpAmAF59V5uZfNdFBFwdAoalEyZZi6d/RCJAPTWF/HsDAlw3+h96ogcgjNK01endxuQDoyTvv
/// uHHvoZh9Lrx5S8m2sQDjdpTiaHRnIGBTCFuUr+MvH8T1JHCD/UmNwgKbtfpclB4DELE0X+oOpYLO
/// L6/uFor3bV65ICk2NnZMZGRkyNBRzF5C8rKMrJxzN56/d9pUce1l2UkUU/iX4Ng/JyHgNVxhtt3T
/// owa1juQmHrsG3Xffg2hbW/K65eJkPpPZzSxyVU1q29vi0xtmifzZ6SB8hTGT5yxO35GVlSVmdYDZ
/// yUibPy12GF+9qKxwysrDV185dGnZltI9cRSI1ksaMXI4DQGqSZPZn2x6cKvldcbGsh+zk9RHNK6t
/// pYLLa6MBhItPPnPqYr7Kt3/vmRvBJO0hk1dmXyh++cVcBk5VX3m/6Pj6WVEUgP+U9RfKHTClUL1k
/// 82gA0dpr3zFOOBUBf1H/MduPvRYRHYg+UGw7snJUChp3AGp9dCg5FGDkyjMVzihK1Uq2TqKAill6
/// vPg/2yaA7P78+Ny6eD+A4SsvvrHfBLLfJOkjmcdPL8Kqf+cjIHoFu/0pZtAF0n0+39SVAv+hdQea
/// Dkqy2WLByLS8cgdyoP7KrwKgxm786+0gXnnNvQOJPuAz69TgV1pWVuQuGgL01P0PFRgFXIGAW+r5
/// oW7aYYqYDuEyNVLS0LqDkezvbVN4QE/KKJDanwOqF7tGAzXtyFN7pLbuyvOLgyBs5Y2BX6z+zs7J
/// NAQl55RjNbLLEBDLZtxVInukWw1N/gIZmnewieu/y5sn+wAVnXb6qf2GxKjKM4eAaItd29lU0mMJ
/// FL3whu3X7K4qSGPy/pFpBVX4wl2JgOfwWp17t8triGGbFZWR29C8dkldspt7ZoUACBccLrHDwmU1
/// +wUw6qAjxtg3Fc6k6OUvbAmLZOdUHvjNPvgIF6VyOQJWj1NnBVLtctFr6gzAoWAl2tdukpfnsR9L
/// 39i0vLKBT1LeXTwHwvY67nP74+J4GJlnTYZF/ujIzwzXYtYUVmGnP3dAQKc/2xDwFP61T/41ajOL
/// eLiP9rWvWl6cWxfny+Sa5++XfDBXDV9nNEOukoggSeJgMFdl+vAy682d0fHqz1+HA0SknnyOff7c
/// BgF/U+xLWzXeTlc9HsSOE5q8FO3rCH2+e3SRCICOXZFTLDMOglx+fr96te7Lw6hVMieErzWHz9tt
/// PKeiKC/IjKfANz5T0oAv0q0QsOAnZtPu86edrlqnnnX2Io3Vuw5T5+uiHbMi2EXeFx28+bbvyMwM
/// gLDLvenwaiy1ptpJgWs7FRJY0Cd/L396ahkDrqAZWTdl+PrcDgHdAWziv0bZrTfmdLZ94Qs8RQM7
/// Vl1VkuzUWAqoyKTMgvKeRrkEtiOvsKgnFTavhkRnzlOi2EFN0JcqWzT1GAHxmUVY6++mCJCq+/Es
/// mWm3y17ksSXOYQfQwM6QsrpEnBZLM1/+pPS8MjaHHaJZ+1Gkm/+7ImLIDScHqiqB+lNfixmWnC3B
/// 1O/OCDgRzG5D7Tf5rwzYWbPXxqOBnQgCWfHxtFgeQOj0dNAp9hl76Cw12wWj7Y6CMBQgMuX4Qxzr
/// 4/YImLuY2fwHdlwLKOIws7lOYY2v01Xz4NSa0dCjhNdkDxx2ScPbE56gGCf59QwEhLPf/7P+9hvt
/// QVZOZzYfAZd5d4kkBgigQASuWgmgRjiuGd+GJyCgXV2Bv3a6Ha/7J5/NmlJ/o4VdoeOUAQEAYqxe
/// b7NBcky7sVOBUDANO/54AgIqga0snrq+/+GT/gAzk+cmhoON0zY/ArbwOSIbLewKraK0qT9oWuZx
/// 6qjxk8pnAcyYNmWTQQP9+40g0mwG/OL7qJI6ga/DAxBwg2LblUOMzRTwHaLUv8k2NinXqIcczluC
/// FnaFJjIACE/Necik7u4x8abKd9/Yd9uQ6G9QBdTJpH7NxroXb3lUv5jClgAPQMDJcGbTCqXGTmCj
/// A6N/bOzMpVKvRJcZhxZ2hXbkv9CNu7lmZvom9butgoV9nXo2Fl58zRTLEWEMzh3jAQj4XzSz+Wy8
/// QUATE9532HphwSlmsz8GLexiJZgZ+aV+twqYaAYBZl983SiR5QDYscMZymEI2DaJ2byBDyajSUNS
/// g+rZVuG3BEExuw2Xt+7es33mdrnGMdxYw89IthpALEQLu1YKkFhAwBPYTrSvsz8CmBdPJIGwn5Dz
/// UEg6xatjk/4jd9MF8pVB0VJyFPwzCJFO3HjATOtvG3UFX4PbI2DNDGZTDt+MRhONGrpe0pDzeEUZ
/// u13TMPwgIY3DIxvVjmuMvf+J25nN+SC0sGv1D5iZYgCGq5qLI3w/KLSvs6UXArQvnpAz8ICQr8sI
/// Wct8JGaGKOp4cOzLVTbzoD5xeCAhi8186WN34GtwewQs/YXZPIQW07mAKUxMGAHsZ0K93QfswMIr
/// sFvraERJawk7JSla2LW6C2aGarETgoevkRm8zn65APbFK8PnEXKgklRooFDKvHTmUAilPYcP+aTK
/// zJxhySvwNbg9AtKS1RnCJtN1AXeaCRGxL16zTQD2w18Dk7WOxsqg7LJCl3lo4f9v78x/mkj/OJ5P
/// MmnK3YBiowSJEpUoElHiKkbEez2WDfpVWVwPvPEW1PVYLwIquuq6EvyB9VpRyWrwYJVo0IVEiSse
/// jYrg0RUIFCG0ff6D7/NMW0TXImgL0/J+JX5mpp0+j/kMfXXmmWeep3O5SnWfuRD44HD+py1AHHi2
/// n54Z4xk7PMD6OfmgiyDvc86DIopb+S/IPwZA2QpYGs3Eo0Llrf2ZGMwtFDBaHmCskSbZV0B4Kg8H
/// uiPDncuj1np9247t+8P53zsC/MCzOo/VubmM/ax6J14wf6QA9mwcSa30P+y1G4dB8QpIieDhIf3T
/// 2p/JlJZnAdtJ9CjWUaZ9Bcjz0e7sgwx3LmZN+ucV8P5wfkIB/CKArfaKM4rhpbeKv5ODLRQg+g7w
/// F8+Q1m4lT6gQh0HxCpC/qS/lh/s+Rk9yo37j6ngWJJ8uyrFhgJZfPa4cbrS++AkCxAgEGyKQ4U5m
/// cV+7T368oF6WlfeHs0Ecbjm0OPDiGkH0MGwMpnmnfhpr4Aedb/UgIwtRVzCmqmFGr6F2/wPre5hw
/// FBSvgCO+THToP/XftwunE4WOm/SNBx3YQZRU+k6O/Oxww9i1G3Y02Tb/i+Wxg/ljkOFO5gll2Xmn
/// OJ5omeUi3no42fNkogP3RMhrPvDyx1fJT/uUT/HtlvSWHSHaVXeAKLVxY/fz/FRgcPrsyXanQixX
/// 78VBUL4CLB36Q7c6sFzLYwcjliLDnU2qWteZFyLR/TF6nAso4IXcoX/qDAeWa3nswP8gMtzZNIUP
/// 7cRRG7ZI/+AQuIACmNyhP8WRvXm3iYaiGrqKDHc6j/2j33VW3WmUjQPgEgoYtIGHPHLg4FKjE3m4
/// Q8+R4c7ngWZUJ43ds5N+RfpdQwFJUfJv9gWHFdson1ek+yPBSuCfwN6dMXxT7XTpKJLvIgo4I4kO
/// nhErHFbsdbmf0XgMF6AM9DGqIx1eaXHvwJtIvasoQC/P/ZXS22HFrhNNAU3qY0iwMjBtkaLudWiN
/// 1YspFhMGuY4CWP91PNyVbww4ArNWzCBQhOnFlUPpcFrecV9JY5am20kk3ZUUsC5YxLAlDir1mjz2
/// wKoQ5FdBHA9QrX7TITU1/RYkraxFxl1KAQ/k3sEZvg7qxjFHjEFi7LYD+VUS7zIDVcucP5Jf9X6t
/// tLgc6XYxBbCBSTy8lhxz8V4piV6l+fQM+VUWjYd604iTTu2udztB5ZlciVS7ngL2e4s/jEXBDpl8
/// PlkrugbO/AbpVRzmgjjJb2GB0Tmll23vR4OPGZBmV1TAG+k3Hp9LjmjD0avEU4IV0nGkV4m8To8g
/// zcIrDY4u9+72/hS4ogQJdlEFsMVB4pfhhxAHnCUu7yb+vFZom5BehfKMW0AVm1HqsAJf58zypx4r
/// bmHqIBdWwAt5oNcK9dc34d2Xh5DRqw4huwrmVc7/NKSZsqfwa58hMt3/LbEvSTHppUiqayuAJYaK
/// oR3SVV/bhmeOkqcUX9etAdlVeLtAyf64HiSFz8ss+LIeA+9KsleO9iD16M2X3yGdrq8AnSR+to39
/// Y7/yZO6QdF8uDc8JuwQVZ9eO8ScKGL0w7WxJW6cBbnyY/8vKb0OIVEMWHC3FqEBuogC20VNMJFAs
/// 7fmqAu+p5KlEJw4wIrkug75g/4LRQUTkETpy5sr0nAt/lej0H57FGWueld7MP30gNXF8WADfUzP0
/// f9vO6XDt71YKaAiaIxaZdP0ryjP0jRY/CrkYM9L1aNJdPfbz0mlRQSrbzOSePj6aoEAfH5/mqcq7
/// D5o4d9PB86W48eeOCmB5lhE+pgZ++XzSpukBog9qtfZHpNaFMVY9u1d4MTf32LFjB+kHHk/lniso
/// efIGV/xurgA2y19cCtQM6PfFY4csUcnDEH+rrUJq3YTWJiUE7qYAQ6h8Fv8qKPIL7xRtp3NisVcq
/// QmahAOB6CmClqs1i8UgTXfclhe2yjDh9W0pHYqEA4IoKYFkk9+q9331Q+x8rNS8jeZyoJ5opyCsU
/// AFxTAewnOi8Wz0OCH7azpPo4lfzR10GRaC6GAoCrKoAtsDTo6Yepf29XQQ9C/eT7gLUDQ9EUCAUA
/// 11WAaZq6QCyNa2heOxoFs1XD5VuJb8K1GC0CCgAurABmnCedllcu+mnPt7GUR6MpRe4N+CwkWIek
/// QgHAlRXA2Boxy7S4GEik8Y/bUEZdqjTYMj1lsf+Qf5FTKAC4uAJYBs20tOgVhdGM+58poWqrt/ch
/// y5Mih6QJaAmEAoDrK4Dd6B5iefrbfHYgTc5rZeyPu8kefjst01UZZlAKHhqDAoA7KIC9HqVKs37x
/// L8aQ37IbnxxMqCytP/XZZ/3hvxrcHY8GQQHATRTATGmqUNsTgxVp/UgVs+vy0xa/8fqbR2cGkN/S
/// v237xNH3mDwGCgBuowDGyqfQzObGwPLjCVoiqd/IifFJs6fGDPIm8hifXmJzQt1Odcg1JBMKAO6k
/// AMYuhVJ8iwHhaktObF0+N27MlFlJqb9ebzFUfNUWb6+djcglFADcTAHMnDuIJp77zJe7eLlas7MG
/// mYQCgPspQJwJjCWfpEK7g4A92dmXQvfjTiAUANxVAYy92htG6on77v7nzmB5TqKWNMmYOwIKAG6t
/// APFbf/R7DUkhk9YeyM69eqvgfM6RLTPC1aSK3lGEQUKhAOD+ChCUnd05OyLQUx5EUqUJnboh+zYm
/// CoACQNdRgI3aVzX45YcCQNdVAIACABQAoAAABQAoAEABAAoAUACAAgAUAKAAAAUAKABAAQAKAFAA
/// gAIAFACgAAAFACgAQAEACgBQAIACABQAoAAABQAoAEABAAoAUACAAgAUAKAAAAUAKABAAQAKAFAA
/// gAIAFACgAAAFACgAQAEACgBQAIACABQAoAAABQAoAEABAAoAUACAAgAUADpfARWXj1gDgAKAuypg
/// 6g6WGc3Y/XjyXBj/g67lPhd6jLWGh0uIps2ODaPNbSzeWmpbdypesXL58qcfvFkQ9KaVzfZVBAUA
/// YE8BTXSLRW/iK+cohbHrIR/sFJBmCzqKFS9k5rVWZn3zWnOprdG8k3HLlDr+6QnPW757Lehlc5n1
/// zZt2ywBQAPgiBTwPZUylA+4AABKtSURBVCzkGV9ZRyWM1dHrFvs8oWJrYGcoU7xS+KqVInNONa82
/// l9rM2Zymj/e37WT+7geT2L40006ZLQq2VwaAAsCXKeAW/2p/K1YGBfFw1aflPlkeJmtgc+khY2Wt
/// lnhJ0jevN5faTL5fj32GDz9g22lbN8sb97TWN4z1H5TZsuCP+W9FAAoA7VFAaRVrKOJLPS03vj3v
/// f0i89mh3dlw+X/4w2RZYYE9+irCQsapfvd4ytjSOv/T0cEYM060evrds8EAjYyuIKIedTlgrFb8v
/// teVVwoGeXqkfXM9bd3omHbNawps1ntGsZWzhYFabRjutZYpwUGy+r4sZD6bsG0TRn6wIQAGgHQqw
/// cYJ/0fxH/iFW88YY2ENf/k0L2s+s4R/qNTPa61e+1qg6wdjDXozdmG6spBp2mYo2PCVxnT4ugQdN
/// EZtSY6dK4+8DpIVP/vNySojJsrJpGGPldJE1em9grILu2soUQd58X9eSuYzdDMBRhAKAwxSQQBXW
/// tXIvnTgpeMkqqZRZAttHuSbd5Bfi7WGb+IV9CtMHPjHPXspYevDRJp3aLNwgrthDlrHSVmrNH0GL
/// Pn6tv601r18K1w/VsVwq4CsBzFqmHOTN5roM9JixU0E4ilAAcJgCuvW1rX23gIdrfoyd9jVbA4vl
/// v/csS357Kv8Bjqtjc5OvfX+Ab44aqGO5w0UzAvErBJYtNx62xOTBzy9qbXpZIY35+H+ivmxZFkiv
/// GEvlZwJjpAa+MtdWphzkzea66iU9M4SdwFGEAoCjFFBKy22r3td4WPk/xhZPZ9bQoIq0nssztmAs
/// y+Bfc82eEnH+bpBW8FP5ZL62aqi8x/CIjxXw+PFjnbWS2dKwC+aP/yfdLacNpoitPI7ezPIXxoiV
/// s7Yy5SA239fFjsRmLr6MgwgFAIcpIINsd/wrSc+vAwL5FXd/0TYohyu0Xn4v6w7/PR5QcIaxl/IP
/// eyO7SE8Zi8nhG6HbDcb8GnaH7DXfF46nSbc+8fqYfHmxdqqImhsVFybueW1kfjX/Wsq0BLH5vi42
/// txhHEAoAjlTAMKq0rhm9HzJjAhdCDZXdqJMDW0jnxDv7w3jcr8rgsckz0/w2/TVbOoBv+OdcYS/o
/// 4km2LZu99Gz6ZH3FQ6TET99UvB7Lzy2MiWtES38V3V3LtJcOMz09umkpUw7yZnNd/Hwg+o8GHEMo
/// ADhKATVriJbZev1c/v5o2n1xwd3zit4S0lQUPDs+Wk3p/OU8+b4hOxcYuc7AWO+NfH0AP0e4p1lo
/// ZAndMxdd+XR9h9dU2vuvHI9du21Zobz6rueo12zw1HpWH5RmLVMO8mZzXezvDTGkycdBhAKAw84C
/// OpPaqZa+v8aSNn4gK5Ox0lh1I44iFADcQAGNI/2Grz79uK5oeXUbP+FRJV8zVOMoQgHADRSQtHDl
/// 2CAi3w2Gtn5iyF885MfhIEIBwC3OAuRQ2Y5PPP9xd9bBTFwHQAHAPdoCABQAoAAABQAoAEABAAoA
/// UACAAgAUAKAAAAUAKABAAQAKAFAAgAIAFACgAAAFACgAQAEACgBQAIACABQAoAAABQAoAEABAAoA
/// UAD4PPm5ublnaTWPuTXIBhQAuhxzyIYnRt6BAkAXPAuwGUCaj2RAAaDrYfSxOeA6kgEFgC7IEsli
/// AI0JuYACQBfklvU6YBVSAQWALkmgxQGYmxMKAF2TDfKVgBaJgAJA16RUvg7YgkRAAaCLEiIcUIY8
/// QAGgi7KDXwn0QxqgANBVecqvA9KRBigAdFnCicqRBSgAdFkyaSiSAAWALojhn4vH96aumUPDl6fs
/// Onq+WI+UQAGgi/DyxOoYDRGpe4ZHjfEbGR3Rx5dvqSLmHSw1IztQAHBr9H8s6kNSxLzMS2X18guP
/// 5Wh8fiN7dYwv+X13CLcIoQDgrtQcjSIpalNBg70dSjO/9aKemx4hVVAAcDuMf8ZJqtkXDZ/ZzVSc
/// qqXIQ2+RMCgAuBOGjG40KufD73/tyycl1/MLisvKa4wtXzf/laiW5uuQNCgAuAvV23091r94v607
/// tzsh0pveI/X7LjWn+L0I6rNDaNYDJA4KAO5AXaqH7/Zq21Z5TkIPouDxq45eKSp99tZQU15WXHDy
/// p+8Hqkg9IaPENnqI6XQYTXuI5EEBwOU5E6jJsF0BPP+5D6lG77r9qeFCzU+OzfAnnwW3bC/khUsp
/// 9cgfFABcGt04SvjXeu2fPZICVhW2Olrwg72DKGirtR3AnKPpkYMUQgHAdTFulcL/tqz+u81bmpxr
/// /PxnyrYF0WTrQEL6H2jSK6QRCgAuSsVw1UHLxX3FClVAem0bP2bKi6RxNy3rN/v6X0MioQDgkhQE
/// hN63XAIkSz0PNbTro9E0ytJP0DCbko3IJRQAXO8iYB0lWlrzTnXXHGtq78dvDZHWW1oRs1QjKpFO
/// KAC4GPUTVMfklUejKan6Cwow/+qjPSevPRzQrRQJhQKAS1Ed5WtpB8xRDbrzpWUk0xz5RMAQ630T
/// KYUCgAtRHhokPwNYN4O2fMVcQVc0of+IZWO8hEnIoQDgOpRpw+R7efdCNFe+qiD9GNUBsTQtomyk
/// FQoArnIOEDi8RizzVKNef2VRxk20TB5IZAudQmKhAOASvO0bJhsgmxY7YMLQ86p4uUPhOukyUgsF
/// ABfAEBH8RiwPUIpDyiv0GlUnlj+qbiO5UABQPMZxmidiuZaOOKjE0m6DxV1F40Q/PDoIBQDFs8RT
/// vom/g844rMhnPaPe8cW7YSEG5BcKAMoml06LxW/0iwML1flPEp2E9d3ikWAoACiap17LxSKPdjq0
/// 2BL1bLG4QVlIMRQAFExjRJh4HOi2tNzBBV+2zEG+WYWuwlAAUDCrPHU8VmknOXxSkGN0lUdTdGgD
/// sgwFAKVSQr+LxVStEwYBn6MRHQ5fem5BmqEAoFDMQ0eIxV6pyAmFG0KjRUejTOkxEg0FAGVyRBIP
/// 9ZRI6U4pvVS1XVwKDIpFoqEAoEj0PqI7oGnwKCeVbzkBuE14aBAKAIpkfk8xStAvkrP68JnCx4jF
/// PG0jcg0FAOVRLokRv/XeG51WQwmd5fGN6jCSDQUA5bE4SPTgm6N14vQf87qJUYiXa5uQbSgAKI03
/// qt94LKXzTqzjrc/PPFZIx5FuKAAojTXdxCX69wOdWsk2X/Gg0PzeGFYcCgAKo1qdyeNjJzfXV3tk
/// 8KijP5BwKAAoi8Nq8fucGGp2bjUbAkQH4ekxSDgUAJRFZCKz3RRwJnr5dsAFqkDGoQCgJJ7QXzxu
/// DXT6RXpSGA9NmjSkHAoASmJTD9GBP3i90ysqIvG48NJQpBwKAEoiSHQIukn3nV9Tn7U8/E0lyDkU
/// AJRDMT3gcUF4B1S1LUBcbISkIulQAFAOu7vz0Oi1vwOqeiqPHbJkKJIOBQDlMEYM7XeDyjuirkEr
/// mRiltAZZhwKAUmhSixn/fgrpkMrWiHsCVXQJaYcCgFK4Sc95HL6wQyq7RHoeB65B2qEAoBS2B/FQ
/// L53ukMoMknhkePVApB0KAEph0iwertCbjqktapHcGIChhKEAoBS0YrTA3UEdVNuaCCYeSELPACgA
/// KIQayucxYUIHVXfMgweTKgeJhwKAMiimpzxGruqg6grpNY8DNiPxUABQBmdJjOTlfbSDqntDhTx+
/// OwuJhwKAMsjQyl/MG/ber103Le679bUfvjh0fWubrSLLJjkKiYcCgDJYI3rr/k0v7LxdFrLRzIyp
/// IQ8+eHWmmBmsktnCzHZMFDZ4Aw97gpF4KAAog8SJPFyj2k+/aw77Rl5OHGD6+K3yEbbQLkYt5iHL
/// C4mHAoAy+DaBh3NkZ7iQbMqTl5cp+6N3XvYPtYb2MUU8kXCeMIYoFACUQYz4Vc5R2RME/Wv5wtNU
/// lkXEDPt5MOcmjmS7yHuxJcibfyZpaxL9Boj7/YfmLJGI7/ZpZk/h4SoZkHkoACiCb5LFt9bfzrt9
/// 1NYVVR/GeouvtQgVxH/8xT9LEJsv1bT7xUkaysuiapZGa+3Vt1jMWlhIb5F5KAAogkjxZU2z1zyn
/// 6mVd6SUxFioUIIcPFSDHvuL1AL7XFH6SX0bD7NW3fggPd6gSmYcCgCIYJhSQ0cveWYBkaQY0SwGf
/// UUDzm4cojz2lTfbqWy3uBxbJHYQAFAA6n5HLeTjqa+fdyda7hS9pXFsVwLJ9103dYbJX34KxPFzn
/// FwsACgBKIFYMFHBCsvPuSesdgd8pS3zBmxjr3qwAMciIHD5UgGmVrrX6ZsQxcYMBzYFQAFAGcTN4
/// yCN78/2Gj5YXg/vzHabTFl2mL101GyiQsRB1hTXIm0FCAT3IyHb0Pn71js7uWcDEuTyclpB4KAAo
/// g6QY+cS8ys7bT7SLG9i7+SHiWSLdUPVY3YiEP2o2EmUaNnY/z5gI78RmOtGuugNEqY0FAcTR2Juk
/// eMQKHn7xR+KhAKAMNokRfErpkb3363aq+/Xa147z9uMZjJkrf7f3JQ/dysN2zCYCBQCFcMiPh3q6
/// aH+Pn0lqxwgf6ZaWvud2piUwSSd5nD8GiYcCgDLIl1vmtHtb2WW7pLnV5vIm025+UXE3vuzTb+vk
/// AYNGL0TioQCgDMrkicRiFrS2j25T1KIjbSyvekWwKur7LHvPAFyiOh6DdyPxUABQBo3y2MFLR3RQ
/// dfsC5euOC0g8FAAUwgAxqegvfh1U2zxxk9EyWBmAAoASmCVGDi2hBx1TW28xukiW2oy8QwFAIRz2
/// 4NftZp+DHVLZC3mEstmxSDsUAJTCQyrmcdq0DqnsuEpMItJjD9IOBQDFEJjGw0FvU0fUNSemWToA
/// CgAKaQwQz+6VdcjX0tx9t+3SA0ABQCFkqRp57LuyA6q6Lrc6xk1G0qEAoBzK6TKPO/w74Kd5rphR
/// sNHrAJIOBQAFETVLFsElp1fU4CXuO5ztqFmMARQA2sRRlZhFIDre6RWdkvQ8Tp6AlEMBQEnUqo4z
/// cb/uX2dXFCMaAfRyj2QABQDlECcG9n7nv8nJ1RRTAY8HPN8h41AAUBQX5T77ezxrnFvNlEgRB/6I
/// hEMBQFmYQpJ4rPPZ6dRayuRxSfKpBAmHAoDCyJLEWOFb/Jw6ru8MMUQZ+2YS0g0FAKVh7CU6BlV5
/// /uTUloBcHm9QEdINBQDFcVAlbtdlSo+dVoM5cqRYxIxGsqEAoDwaAtaLNoFBznuK97D0kImZxK4j
/// 2VAAUCCHJDGM+G35ZN0Z6H1ShWPCMXQwFAAUiSk8Wizm9ahyTvkzetbz+Iv0CKmGAoAiKaEzPFb3
/// dE6DfZbcK0jvvRmJhgKAQlnYTTwpUCxlOKHsByr5XsMc+VQAQAFAiVRpFolFhuT4u3b1oSPFk8hX
/// 6U+kGQoAiiXX0hY4SfvawQWb4zWv+OKV/2wkGQoACmaZWnQLqOobVuvYcldJYthgc0xIHXIMBQAF
/// 0zg4TAwhVtkzyqHP8qXRWbH4SXUPKYYCgKLReS4TizLfbx04iNgJkscJu0a/IcFQAFB8c4D8dS1S
/// zXKYA05I8s2AUq9ZSC8UABTPIcoRiwL1eAfdvttHKWLxvHt0I7ILBQDls04SwwmzEv8hDhlGbBvJ
/// 3Qze9g2rQW6hAOAKJKjlOUWeBIXqvrqshjnSKbE0RPTGoMFQAHANjBO9b4rlq8GeX/vI0OOB3tfk
/// c4ChATokFgoALkJjvCR/9xuTKeGrbg6e84p4KsskLOgx0goFAJfBtJiOySu5XuFlX1xK3QJa2SRW
/// yrT9K5FUKAC4Ettol7x8GimlfOGdgTOBmjx55abPSLQEQgHAxfhVmlItluYcTWDOF3xeN44SLHcU
/// 0qU43A2EAoDLURyktTwwqE+k2PY+OliZLIX/La+9nShtMyObUABwPaom2768RSMouqAdn3yWJGl/
/// NdlEchuphAKAa7JXirY2Bt4cS5Gn2nhzoChB6p0lNwOy2mSahmYAKAC4LHcjpXXWuUVKpksec298
/// 9pRetzWYBp2wPl5wspvmOJIIBQAXxvybr9bWPUh/MIJ6Lj1nf3DRppvbhlLg2vvWzbJoWlSNFEIB
/// wLWpTqaBObYf/7LNQ4jC1/xx7+P7hEZd/p6xKgqaf9Vk2zVBCr+D9EEBwPUpmUZhp23fbFbz58ow
/// iajnmPlrt2YcO52VuStl0fRQ/op2Vvbz91cQ39GAUybkDgoAbsGDWRRyQN/iN/9xXvq82KH9e/mQ
/// Z2CfiNHxP50qaTEdadOFCRRxAXcCoQDgPuiSPKVJZ9vUw+fOEl+KvYKUQQHAvWg4PYG8En9/2epO
/// 9Vc2hFD/9JdIFxQA3JA3+0dK1Hdxbvkn3625sfUbifqtvotEQQHAbXlXsHGYRF7D527PuVLy7G0N
/// p/yfwj/2LhuvJQpecArDgkABwO0xFB1dMq6PRC3o8c0P6fk4/YcCQFei7llJYQGn5NG/aPyHAgAA
/// UAAAAAoAAEABAAC34P9sP66p30fJoQAAAABJRU5ErkJggg==">
/// </div>
#[macro_export]
macro_rules! stm {
    (@widen_enum_variant noargs, $tuple:expr, ($($idx:tt),*), ($($arg:tt),* $(,)?) -> $enum_name:ident :: $start:ident ($($comp:expr),*)) => {
        $enum_name :: $start($($comp),*)
    };
    (@widen_enum_variant args, $tuple:expr, ($($idx:expr),*), () -> $enum_name:ident :: $start:ident ($($comp:expr),*)) => {
        $enum_name :: $start ($($comp),*)
    };
    (@widen_enum_variant args, $tuple:expr, ($head_idx:tt, $($idx:tt),*), ($head:tt $(, $arg:tt)* $(,)?) -> $enum_name:ident :: $start:ident ()) => {
        crate::stm!(@widen_enum_variant args, $tuple,  ($($idx),*), ($($arg),*) -> $enum_name :: $start ( $tuple.$head_idx ))
    };
    (@widen_enum_variant args, $tuple:expr, ($head_idx:tt, $($idx:tt),*), ($head:tt $(, $arg:tt)* $(,)?) -> $enum_name:ident :: $start:ident ($($comp:expr),+)) => {
        crate::stm!(@widen_enum_variant args, $tuple, ($($idx),*), ($($arg),*) -> $enum_name :: $start ( $($comp),*, $tuple.$head_idx ))
    };
    (@sub_build_enum () -> { pub enum $enum_name:ident {$($processed_var:ident(dropper::$processed:ident)),*}}) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $($processed_var(dropper::$processed)),*
        }
    };
    (@sub_build_enum ($head:tt |end| $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident { } }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name {
                $head(dropper::$head)
            }
        });
    };
    (@sub_build_enum ($head:tt |end| $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident { $($processed_var:ident( dropper::$processed:ident)),+} }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name {
                $($processed_var(dropper::$processed)),*,
                $head(dropper::$head)
            }
        });
    };
    (@sub_build_enum ($head:tt $(, $tail:ident $(| $tag:ident |)?)*)-> { pub enum $enum_name:ident { $($processed_var:ident( dropper::$processed:ident)),*} }) => {
        crate::stm!(@sub_build_enum ($($tail $(| $tag |)*),*) -> {
            pub enum $enum_name {
                $($processed_var(dropper::$processed)),*
            }
        });
    };
    (@sub_bare_dropper_enum $enum_name:ident, $($var:ident),*) => {
        #[allow(dead_code)]
        #[derive(Debug)]
        pub enum $enum_name {
            $(
                $var(dropper::$var),
            )*
        }
    };
    (@sub_wall nowall $stripped_name:ident $term_name:ident $mod_name:ident, $enum_name:ident, $($var:ident($($arg:ty),*)),*) => {
        #[allow(dead_code)]
        pub enum $enum_name {
            $(
                $var($mod_name::$var $(, $arg )*),
            )*
        }
    };
    (@sub_wall wall $stripped_name:ident $term_name:ident $mod_name:ident, $enum_name:ident, $($var:ident($($arg:ty),*)),*) => {
        #[warn(dead_code)]
        pub enum $enum_name {
            $(
                $var($mod_name::$var $(, $arg )*),
            )*
        }
    };

    (@sub_end_filter end $($sub:tt)*) => {$($sub)*};
    (@sub_end_filter $tag:tt $($sub:tt)*) => {};

    (@sub_pattern $_t:tt $sub:pat) => {$sub};

    (@insert_tuple_params noargs, ($($start_arg:ty),*)) => (
        ($($start_arg),*)
    );
    (@insert_tuple_params args, ($($start_arg:ty),*)) => (
        ($($start_arg),*,)
    );

    (@private $machine_tag:tt $pertinence:tt $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start_trailing:tt $start: ident($($start_arg:ty),* ,) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {

        use $mod_name::$term_name;
        use $mod_name::$stripped_name;

        mod $mod_name
        {
            pub struct $start {
                pub finaliser: Option<Box<dyn FnOnce($stripped_name) -> $term_name>>
            }

            impl Drop for $start {
                fn drop(&mut self) {
                    if let Some(finaliser)=self.finaliser.take() {
                        let _term=(finaliser)($stripped_name::$start(dropper::$start::new()));
                    }
                }
            }

            $(
                impl From<$start_e> for $start {
                    fn from(mut old_st: $start_e) -> $start {
                        use log::trace;

                        trace!("{:?} -> {:?}", stringify!($start_e), stringify!($start));
                        $start {
                            finaliser: old_st.finaliser.take()
                        }
                    }
                }
            )*

            impl std::fmt::Debug for $start {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                    f.debug_struct(stringify!($start))
                        .finish()
                }
            }

            impl $start {
                pub fn end_tags_found(&self){}

                #[allow(dead_code, unreachable_code)]
                pub fn is_accepting_state(&self) -> bool {
                    $( crate::stm!{@sub_end_filter $start_tag
                                   return true;
                    } )*
                    return false;
                }

            }

            $(
                impl std::fmt::Debug for $node {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                        f.debug_struct(stringify!($node))
                            .finish()
                    }
                }

                pub struct $node {
                    pub finaliser: Option<Box<dyn FnOnce($stripped_name) -> $term_name>>,
                    _secret: ()
                }

                impl $node {
                    #[allow(dead_code, unreachable_code)]
                    pub fn is_accepting_state(&self) -> bool {
                        $( crate::stm!{@sub_end_filter $tag
                                       return true;
                        } )*;
                        return false;
                    }
                }

                impl Drop for $node {
                    fn drop(&mut self) {
                        if let Some(finaliser)=self.finaliser.take() {
                            let _term=(finaliser)($stripped_name::$node(dropper::$node::new()));
                        }
                    }
                }

                $(
                    impl From<$e> for $node {
                        fn from(mut old_st: $e) -> $node {
                            println!("{:?} -> {:?}", stringify!($e), stringify!($node));
                            $node {
                                finaliser: old_st.finaliser.take(),
                                _secret: ()
                            }
                        }
                    }
                )*
            )*

            #[cfg(feature = "render_stm")]
            pub type Nd = &'static str;
            #[cfg(feature = "render_stm")]
            pub type Ed=(&'static str, &'static str);

            #[cfg(feature = "render_stm")]
            pub struct MachineEdges(pub Vec<Ed>);

            #[cfg(feature = "render_stm")]
            pub const START_NODE_NAME:&str="_start";

            #[cfg(feature = "render_stm")]
            impl<'a> dot::GraphWalk<'a, Nd, Ed> for MachineEdges {
                fn nodes(&self) -> dot::Nodes<'a, Nd> {
                    // (assumes that |N| \approxeq |E|)
                    let &MachineEdges(ref v) = self;
                    let mut nodes = Vec::with_capacity(v.len()*2);
                    nodes.push(START_NODE_NAME);
                    for &(s,t) in v {
                        nodes.push(s); nodes.push(t);
                    }
                    nodes.sort();
                    nodes.dedup();

                    std::borrow::Cow::Owned(nodes)
                }

                fn edges(&'a self) -> dot::Edges<'a, Ed> {
                    let &MachineEdges(ref edges) = self;
                    std::borrow::Cow::Borrowed(&edges[..])
                }

                fn source(&self, e: &Ed) -> Nd { e.0 }
                fn target(&self, e: &Ed) -> Nd { e.1 }
            }

            #[cfg(feature = "render_stm")]
            impl<'a> dot::Labeller<'a, Nd, Ed> for MachineEdges {
                fn graph_id(&'a self) -> dot::Id<'a> { dot::Id::new(stringify!($mod_name)).unwrap() }

                fn node_shape(&'a self, node: &Nd) -> Option<dot::LabelText<'a>> {
                    if &START_NODE_NAME==node {
                        Some(dot::LabelText::LabelStr("point".into()))
                    } else {
                        #[allow(unused_mut)]
                        let mut shape=Some(dot::LabelText::LabelStr("ellipse".into()));
                        if node==&stringify!($start) {
                            $( crate::stm!(@sub_end_filter $start_tag {
                                shape=Some(dot::LabelText::LabelStr("doublecircle".into()));
                            } ) )*
                        }
                        $(
                            if node==&stringify!($node) {
                                $( crate::stm!(@sub_end_filter $tag {
                                    shape=Some(dot::LabelText::LabelStr("doublecircle".into()));

                                } ) )*
                            }
                        )*
                        shape
                    }
                }

                fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
                    dot::Id::new(*n).unwrap()
                }

                #[allow(unused_mut, unused_variables)]
                fn node_label(&'a self, node: &Nd) -> dot::LabelText<'a> {
                    let mut last: Option<char>=None;
                    let mut rows=1.0;
                    let mut cols=0.0;
                    let mut name=String::new();
                    for ch in node.chars() {
                        if let Some(last)=last {
                            $(
                                if node==&stringify!($start) {
                                    $( crate::stm!(@sub_end_filter $tag {
                                        if last.is_lowercase() && ch.is_uppercase() && cols>3.0+1.25*rows {
                                            name.push('\n');

                                            rows+=1.0;
                                            cols=0.0;
                                        }
                                    } ) )*
                                }
                                if node==&stringify!($node) {
                                    $( crate::stm!(@sub_end_filter $tag {
                                        if last.is_lowercase() && ch.is_uppercase() && cols>3.0+1.25*rows {
                                            name.push('\n');

                                            rows+=1.0;
                                            cols=0.0;
                                        }
                                    } ) )*
                                }
                            )*
                        }

                        cols+=1.0;
                        name.push(ch);
                        last=Some(ch);
                    }

                    dot::LabelText::LabelStr(name.into())
                }

                fn edge_label(&'a self, (f, to): &Ed) -> dot::LabelText<'a> {
                    {
                        let dest_name=stringify!($start);
                        if &dest_name==to {
                            let mut edge_name=if START_NODE_NAME==*f {
                                String::from(format!("<TABLE BORDER=\"0\"><TR><TD><B><I> -&gt; {:?}</I></B></TD></TR>", to.replace("<", "&lt;").replace(">", "&gt;")))
                            } else {
                                String::from(format!("<TABLE BORDER=\"0\"><TR><TD><I>{:?} -&gt; {:?}</I></TD></TR>", f.replace("<", "&lt;").replace(">", "&gt;"), to.replace("<", "&lt;").replace(">", "&gt;")))
                            };
                            edge_name.push_str(""); //to avoid warning about edge_name not needing to be mutable
                            $(
                                let arg=stringify!($start_arg);
                                let arg_line=format!("<TR><TD>{}</TD></TR>", arg.replace("<", "&lt;").replace(">", "&gt;"));
                                (&mut edge_name).push_str(&arg_line);
                            )*;
                            return dot::LabelText::HtmlStr(format!("{}</TABLE>", edge_name).into())
                        }
                    }
                    $(
                        {
                            let dest_name=stringify!($node);
                            if &dest_name==to {
                                let mut edge_name=String::from(format!("<TABLE BORDER=\"0\"><TR><TD><I>{:?} -&gt; {:?}</I></TD></TR>", f.replace("<", "&lt;").replace(">", "&gt;"), to.replace("<", "&lt;").replace(">", "&gt;")));
                                edge_name.push_str(""); //to avoid warning about edge_name not needing to be mutable
                                $(
                                    let arg=stringify!($arg);
                                    let arg_line=format!("<TR><TD>{}</TD></TR>", arg.replace("<", "&lt;").replace(">", "&gt;"));
                                    (&mut edge_name).push_str(&arg_line);
                                )*;
                                return dot::LabelText::HtmlStr(format!("{}</TABLE>", edge_name).into())
                            }
                        }
                    )*;
                    dot::LabelText::EscStr("".into())
                }
            }

            mod dropper {
                #[derive(Debug)]
                pub struct $start{
                    _secret: ()
                }

                impl $start {
                    pub fn new() -> $start {
                        $start {
                            _secret: ()
                        }
                    }

                    #[allow(dead_code, unreachable_code)]
                    pub fn is_accepting_state(&self) -> bool {
                        $( crate::stm!{@sub_end_filter $start_tag
                                       return true;
                        } )*
                        return false;
                    }
                }

                $(
                    #[derive(Debug)]
                    pub struct $node {
                        _secret: ()
                    }

                    impl $node {
                        pub fn new() -> $node {
                            $node {
                                _secret: ()
                            }
                        }

                        #[allow(dead_code, unreachable_code)]
                        pub fn is_accepting_state(&self) -> bool {
                            $( crate::stm!{@sub_end_filter $tag
                                           return true;
                            } )*;
                            return false;
                        }
                    }
                )*

                $(
                    impl From<$start_e> for $start {
                        fn from(_old_st: $start_e) -> $start {
                            $start{
                                _secret: ()
                            }
                        }
                    }
                )*

                $(
                    $(
                        impl From<$e> for $node {
                            fn from(_old_st: $e) -> $node {
                                $node{ _secret: ()}
                            }
                        }
                    )*
                )*
            }

            crate::stm!(@sub_build_enum (
                $start $(| $start_tag |)*,
                $($node $(| $tag |)*),*
            ) -> {
                pub enum $term_name {
                }
            });

            crate::stm!(@sub_bare_dropper_enum $stripped_name, $start,$($node),*);
        }

        stm!(@sub_wall $machine_tag $stripped_name $term_name $mod_name, $enum_name, $start($($start_arg),*),$($node($($arg),*)),*);

        impl $stripped_name {
            #[allow(dead_code)]
            pub fn at_accepting_state(&self) -> bool {
                match self {
                    $stripped_name::$start(st) =>
                        st.is_accepting_state(),
                    $(
                        $stripped_name::$node(st) =>
                            st.is_accepting_state(),
                    )*
                }
            }
        }

        impl std::fmt::Debug for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
                f.debug_struct(stringify!($enum_name))
                    .field("state", &self.state().to_string())
                    .finish()
            }
        }

        impl $enum_name {
            #[allow(unused_variables)]
            pub fn new(arg : crate::stm!(@insert_tuple_params $start_trailing, ($($start_arg),*)), finaliser: Box<dyn FnOnce($mod_name::$stripped_name) -> $mod_name::$term_name>) -> $enum_name {
                let node=$mod_name::$start {
                    finaliser: Some(finaliser)
                };

                $( crate::stm!{@sub_end_filter $start_tag
                               node.end_tags_found();
                } )*;

                $(
                    $( crate::stm!{@sub_end_filter $tag
                                   node.end_tags_found();
                    } )*
                )*;

                crate::stm!(@widen_enum_variant $start_trailing, arg, (0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31), ($($start_arg,)*) -> $enum_name::$start(node))
            }

            #[allow(dead_code)]
            pub fn at_accepting_state(&self) -> bool {
                match self {
                    $enum_name::$start(st $(, stm!(@sub_pattern ($start_arg) _ ))*) =>
                        st.is_accepting_state(),
                    $(
                        $enum_name::$node(st $(, stm!(@sub_pattern ($arg) _))*) =>
                            st.is_accepting_state(),
                    )*
                }
            }

            #[allow(dead_code)]
            pub fn state(&self) -> &'static str {
                match self {
                    $enum_name::$start(_st $(, stm!(@sub_pattern ($start_arg) _ ))*) => stringify!($start),
                    $(
                        $enum_name::$node(_st $(, stm!(@sub_pattern ($arg) _))*) => stringify!($node),
                    )*
                }
            }

            #[allow(unused_variables)]
            pub fn render_to<W: std::io::Write>(output: &mut W) {
                #[cfg(feature = "render_stm")]
                {
                    let mut edge_vec=Vec::new();
                    edge_vec.push(($mod_name::START_NODE_NAME, stringify!($start)));

                    $(
                        edge_vec.push({
                            let f=stringify!($start_e);
                            let t=stringify!($start);
                            (f,t)
                        });
                    )*;

                    $(
                        $(
                            edge_vec.push({
                                let f=stringify!($e);
                                let t=stringify!($node);
                                (f,t)
                            });
                        )*
                    )*;

                    let edges = $mod_name::MachineEdges(edge_vec);
                    dot::render(&edges, output).unwrap()
                }
            }
        }
    };

    (states $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private nowall ignorable $mod_name, $enum_name, $stripped_name, $term_name, ||{}, $term_name, [$($start_e),*] => noargs $start( , ) $(|$start_tag|)*, {
            $([$($e),*] => $node() $(|$tag|)*);* });
    };
    (machine $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident($($start_arg:ty),+  $(,)?) $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall ignorable $mod_name, $enum_name, $stripped_name, $term_name, [$($start_e), *] => args $start($($start_arg),*,) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
    (machine $mod_name:ident, $enum_name:ident, $stripped_name:ident, $term_name:ident, [$($start_e:ident), *] => $start: ident() $(| $start_tag:tt |)?, { $( [$($e:ident), +] => $node:ident($($arg:ty),*) $(| $tag:tt |)? );+ $(;)? } ) => {
        stm!(@private wall ignorable $mod_name, $enum_name, $stripped_name, $term_name, [$($start_e), *] => noargs $start( , ) $(| $start_tag|)*, { $( [$($e), *] => $node($($arg),*) $(|$tag|)* );* } );
    };
}
