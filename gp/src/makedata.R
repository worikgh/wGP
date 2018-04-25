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

f <- 0.01*m[,1]^2 + m[,2]^3 + 1.3*m[,1]
d <- cbind(m,f)
colnames(d) <- c('a', 'b', 'c')
write.csv(file='Data.in', d, row.names=FALSE)
