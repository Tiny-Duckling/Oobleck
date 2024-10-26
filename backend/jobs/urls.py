from django.urls import path, include
from rest_framework.routers import DefaultRouter
from .views import OpeningViewSet

router = DefaultRouter()
router.register(r'opening', OpeningViewSet)

urlpatterns = [
    path('', include(router.urls)),
]
