

fn test() {

    let mut bb = NextBuilder::new().with_number(1);

}



pub struct NextBuilder {

}


impl NextBuilder {

    pub fn new() -> Self {
        NextBuilder {

        }
    }

    pub fn with_number(self, n: usize) -> Self {

        self
    }



}