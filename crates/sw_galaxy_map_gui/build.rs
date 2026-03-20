fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/sw_galaxy_map.ico");
        res.compile().expect("Failed to embed Windows icon");
    }
}
