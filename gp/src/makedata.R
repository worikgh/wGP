c <- seq(-1,1,by=0.01)
d <- c
m <- matrix(,ncol=2, nrow=1+length(d)^2)
r <- 1 
for (c1 in 1:length(c)){
    for ( d1 in 1:length(d) ){
        m[r,] <- c(c[c1], d[d1])
        r <- r+1
    }
}
## Add Add Multiply "x" "x" "x" Float 0.2021075922018165
x <- m[,1]
g <- (x*x)+x+0.2021075922018165
#f <- 0.01*m[,1]^2 + m[,2]^3 + 1.3*m[,1]
f <- m[,1]^2+m[,1]+1
# Add Add Multiply x x x 1
d <- cbind(m[,1],f)
colnames(d) <- c('x', 'y')
write.csv(file='Data.in', d, row.names=FALSE)
