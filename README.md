# sw_galaxy_map

**sw_galaxy_map** is a command-line application written in Rust that allows querying
and exploring the Star Wars galaxy using a local SQLite database.

The application provides tools to:

- search for planets by name or alias,
- display all available information about a specific planet,
- find nearby planets within a given radius using Euclidean distance
  on X/Y coordinates expressed in parsecs.

The project is designed as an offline, fast, and script-friendly CLI tool,
intended primarily for educational and non-commercial use.

---

## Acknowledgements

The planetary data used by this project were obtained from the **Star Wars Galaxy Map**
available at:

[Star Wars Galaxy Map](http://www.swgalaxymap.com/): Explore the Galaxy Far, Far Away

The Star Wars Galaxy Map project is created and maintained by **Henry Bernberg**.
All credit for the original dataset, research, and compilation goes to him.

If you find this data valuable, please consider supporting the original author via one
of the official donation channels:

- [Ko-fi](https://ko-fi.com/J3J0197XZ)
- [PayPal](https://www.paypal.com/donate?token=rk-LV-u5miGM2sumnvRL5ZiAFjnwIhhLnsSe-mqEnFgDAmeIhBkG6CQamxUxUoR18iwI0mA8h5ruuIk_)

This project uses the data for **educational and non-commercial purposes** only and
is not affiliated with, endorsed by, or associated with the Star Wars Galaxy Map
website, Lucasfilm, or The Walt Disney Company.
