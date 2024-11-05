from django.urls import path, include
from rest_framework.routers import DefaultRouter
from .views import OpeningViewSet, ApplicationViewSet

router = DefaultRouter()
router.register(r"opening", OpeningViewSet)
router.register(r"application", ApplicationViewSet)

urlpatterns = [
    path("", include(router.urls)),
]
