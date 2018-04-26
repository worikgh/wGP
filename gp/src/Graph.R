data <- read.table('Simulations.txt', header=TRUE)
names(data)
summary(data)
plot(data[,"X0"])
n <- 2
points(data[,2], col=2)
points(data[,3], col=2)
points(data[,4], col=2)
points(data[,5], col=2)
points(data[,6], col=2)
points(data[,7], col=2)
points(data[,8], col=2)
points(data[,9], col=2)
points(data[,10], col=2)

for (c in names(data)) {
    points(data[,c], col=n)
    n <- n+1
}
summary(data[,-1] - data[,1])

data2 <- read.table('/tmp/g.txt', header=FALSE)
